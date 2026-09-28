//! Bounded input for the development CLI's sole, unbuffered stdin consumer.

use std::os::fd::AsFd;
use std::sync::atomic::{AtomicBool, Ordering};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::io::{Errno, read};

use super::{LinuxDevelopmentBoundaryError, LinuxDevelopmentBoundaryErrorKind, development_error};

const MAX_CONFIRMATION_BYTES: usize = 256;
const INPUT_POLL_TIME: Timespec = Timespec {
    tv_sec: 0,
    tv_nsec: 50_000_000,
};

/// Closed confirmation vocabulary of the explicit development CLI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxDevelopmentConfirmationKind {
    /// Confirm the displayed exact protected operation challenge.
    OperationApproval,
    /// Confirm the displayed exact bounded session preauthorization contract.
    SessionPreauthorization,
}

impl LinuxDevelopmentConfirmationKind {
    const fn word(self) -> &'static str {
        match self {
            Self::OperationApproval => "yes",
            Self::SessionPreauthorization => "preauthorize",
        }
    }
}

/// Non-authoritative local input; the protected host still owns any exact grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxDevelopmentConfirmation {
    /// A complete bounded line matches the closed confirmation word.
    Confirmed,
    /// A different complete line or EOF supplies no confirmation.
    Declined,
    /// The existing signal owner's flag is set and remains unconsumed.
    Cancelled,
}

/// Reads one bounded confirmation from the development CLI's standard input.
///
/// This entry point is for the dedicated CLI, whose confirmation paths are its
/// sole stdin consumers. They must not mix buffered standard-input reads with
/// this direct descriptor reader. It does not change flags or terminal settings,
/// seek, consume a later line, or return input bytes beyond this boundary.
pub fn read_development_confirmation(
    kind: LinuxDevelopmentConfirmationKind,
    cancellation: &AtomicBool,
) -> Result<LinuxDevelopmentConfirmation, LinuxDevelopmentBoundaryError> {
    let stdin = std::io::stdin();
    let _input_owner = stdin.lock();
    read_confirmation(&stdin, kind, cancellation)
}

fn input_error() -> LinuxDevelopmentBoundaryError {
    development_error(LinuxDevelopmentBoundaryErrorKind::ConfirmationInputFailed)
}

fn read_confirmation(
    input: &impl AsFd,
    kind: LinuxDevelopmentConfirmationKind,
    cancellation: &AtomicBool,
) -> Result<LinuxDevelopmentConfirmation, LinuxDevelopmentBoundaryError> {
    if cancellation.load(Ordering::Acquire) {
        return Ok(LinuxDevelopmentConfirmation::Cancelled);
    }
    // A write-only pipe can wait forever for readable data while its peer stays
    // open. Refuse its access mode without changing the shared descriptor flags.
    if rustix::fs::fcntl_getfl(input)
        .map_err(|_| input_error())?
        .contains(rustix::fs::OFlags::WRONLY)
    {
        return Err(input_error());
    }
    let mut bytes = [0_u8; MAX_CONFIRMATION_BYTES];
    let mut length = 0;
    loop {
        if cancellation.load(Ordering::Acquire) {
            return Ok(LinuxDevelopmentConfirmation::Cancelled);
        }
        let mut descriptors = [PollFd::new(input, PollFlags::IN)];
        match poll(&mut descriptors, Some(&INPUT_POLL_TIME)) {
            Ok(0) | Err(Errno::INTR) => continue,
            Ok(_) => {}
            Err(_) => return Err(input_error()),
        }
        if cancellation.load(Ordering::Acquire) {
            return Ok(LinuxDevelopmentConfirmation::Cancelled);
        }
        let ready = descriptors[0].revents();
        if ready.intersects(PollFlags::ERR | PollFlags::NVAL)
            || !ready.intersects(PollFlags::IN | PollFlags::HUP)
        {
            return Err(input_error());
        }
        // One byte prevents read-ahead from consuming any future challenge's
        // answer. Polling does not change a shared open-file description's flags.
        match read(input, &mut bytes[length..length + 1]) {
            Ok(0) => return Ok(LinuxDevelopmentConfirmation::Declined),
            Ok(1) => length += 1,
            Ok(_) => return Err(input_error()),
            Err(Errno::INTR | Errno::AGAIN) => continue,
            Err(_) => return Err(input_error()),
        }
        if cancellation.load(Ordering::Acquire) {
            return Ok(LinuxDevelopmentConfirmation::Cancelled);
        }
        if bytes[length - 1] == b'\n' {
            let line = std::str::from_utf8(&bytes[..length]).map_err(|_| input_error())?;
            return Ok(if line.trim() == kind.word() {
                LinuxDevelopmentConfirmation::Confirmed
            } else {
                LinuxDevelopmentConfirmation::Declined
            });
        }
        if length == bytes.len() {
            return Err(input_error());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn closed_line(
        bytes: &[u8],
        kind: LinuxDevelopmentConfirmationKind,
    ) -> Result<LinuxDevelopmentConfirmation, LinuxDevelopmentBoundaryError> {
        let (input, mut output) = std::io::pipe().unwrap();
        output.write_all(bytes).unwrap();
        drop(output);
        read_confirmation(&input, kind, &AtomicBool::new(false))
    }

    #[test]
    fn closed_words_require_a_complete_line_and_preserve_whitespace_semantics() {
        use LinuxDevelopmentConfirmation::{Confirmed, Declined};
        use LinuxDevelopmentConfirmationKind::{OperationApproval, SessionPreauthorization};
        for (bytes, kind, expected) in [
            (&b"yes\n"[..], OperationApproval, Confirmed),
            (&b" \tyes\r\n"[..], OperationApproval, Confirmed),
            (&b"preauthorize\n"[..], SessionPreauthorization, Confirmed),
            (&b"yes\n"[..], SessionPreauthorization, Declined),
            (&b"preauthorize\n"[..], OperationApproval, Declined),
            (&b"YES\n"[..], OperationApproval, Declined),
            (&b"yes extra\n"[..], OperationApproval, Declined),
            (&b"yes\0\n"[..], OperationApproval, Declined),
            (&b"\n"[..], OperationApproval, Declined),
        ] {
            assert_eq!(closed_line(bytes, kind).unwrap(), expected);
        }
        assert_eq!(
            closed_line("\u{2003}yes\u{2003}\n".as_bytes(), OperationApproval).unwrap(),
            Confirmed
        );
    }

    #[test]
    fn eof_never_confirms_an_approving_fragment() {
        for (bytes, kind) in [
            (
                &b""[..],
                LinuxDevelopmentConfirmationKind::OperationApproval,
            ),
            (
                &b"yes"[..],
                LinuxDevelopmentConfirmationKind::OperationApproval,
            ),
            (
                &b"yes "[..],
                LinuxDevelopmentConfirmationKind::OperationApproval,
            ),
            (
                &b"preauthorize"[..],
                LinuxDevelopmentConfirmationKind::SessionPreauthorization,
            ),
        ] {
            assert_eq!(
                closed_line(bytes, kind).unwrap(),
                LinuxDevelopmentConfirmation::Declined
            );
        }
    }

    #[test]
    fn limit_requires_the_newline_and_never_accepts_an_overlong_approving_prefix() {
        let kind = LinuxDevelopmentConfirmationKind::OperationApproval;
        let mut exact = b"yes".to_vec();
        exact.resize(MAX_CONFIRMATION_BYTES - 1, b' ');
        exact.push(b'\n');
        assert_eq!(
            closed_line(&exact, kind).unwrap(),
            LinuxDevelopmentConfirmation::Confirmed
        );

        let (input, mut output) = std::io::pipe().unwrap();
        let flags = rustix::fs::fcntl_getfl(&input).unwrap();
        let mut excessive = b"yes".to_vec();
        excessive.resize(MAX_CONFIRMATION_BYTES, b' ');
        excessive.extend_from_slice(b"\nyes\n");
        output.write_all(&excessive).unwrap();
        // The writer stays open: neither EOF nor unbounded draining may be needed.
        assert_eq!(
            read_confirmation(&input, kind, &AtomicBool::new(false))
                .unwrap_err()
                .kind(),
            LinuxDevelopmentBoundaryErrorKind::ConfirmationInputFailed
        );
        assert_eq!(rustix::fs::fcntl_getfl(&input).unwrap(), flags);
        let mut remaining = [0; 5];
        assert_eq!(read(&input, &mut remaining).unwrap(), 5);
        assert_eq!(&remaining, b"\nyes\n");
    }

    #[test]
    fn invalid_utf8_is_a_content_free_failure() {
        for bytes in [&b"\xff\n"[..], &b"yes\xc3\n"[..]] {
            let error = closed_line(bytes, LinuxDevelopmentConfirmationKind::OperationApproval)
                .unwrap_err();
            assert_eq!(
                error.kind(),
                LinuxDevelopmentBoundaryErrorKind::ConfirmationInputFailed
            );
            assert_eq!(
                error.kind().code(),
                "linux.development.confirmation-input.failed"
            );
        }
    }

    #[test]
    fn consecutive_prompts_consume_only_their_own_line_and_leave_flags_unchanged() {
        let (input, mut output) = std::io::pipe().unwrap();
        let flags = rustix::fs::fcntl_getfl(&input).unwrap();
        output.write_all(b"yes\npreauthorize\ntail").unwrap();
        for kind in [
            LinuxDevelopmentConfirmationKind::OperationApproval,
            LinuxDevelopmentConfirmationKind::SessionPreauthorization,
        ] {
            assert_eq!(
                read_confirmation(&input, kind, &AtomicBool::new(false)).unwrap(),
                LinuxDevelopmentConfirmation::Confirmed
            );
            assert_eq!(rustix::fs::fcntl_getfl(&input).unwrap(), flags);
        }
        let mut tail = [0; 4];
        assert_eq!(read(&input, &mut tail).unwrap(), 4);
        assert_eq!(&tail, b"tail");
    }

    #[test]
    fn pending_cancellation_consumes_no_input_and_never_clears_the_flag() {
        let (input, mut output) = std::io::pipe().unwrap();
        output.write_all(b"yes\n").unwrap();
        let flags = rustix::fs::fcntl_getfl(&input).unwrap();
        let requested = AtomicBool::new(true);
        assert_eq!(
            read_confirmation(
                &input,
                LinuxDevelopmentConfirmationKind::OperationApproval,
                &requested
            )
            .unwrap(),
            LinuxDevelopmentConfirmation::Cancelled
        );
        assert!(requested.load(Ordering::Acquire));
        assert_eq!(rustix::fs::fcntl_getfl(&input).unwrap(), flags);
        let mut line = [0; 4];
        assert_eq!(read(&input, &mut line).unwrap(), 4);
        assert_eq!(&line, b"yes\n");
    }

    #[test]
    fn idle_and_partial_open_pipes_cancel_before_test_cleanup_closes_the_writer() {
        use std::sync::Arc;
        use std::time::{Duration, Instant};
        for prefix in [&b""[..], &b"ye"[..]] {
            let (input, mut output) = std::io::pipe().unwrap();
            output.write_all(prefix).unwrap();
            let flags = rustix::fs::fcntl_getfl(&input).unwrap();
            let requested = Arc::new(AtomicBool::new(false));
            let finished = Arc::new(AtomicBool::new(false));
            let fallback = Arc::new(AtomicBool::new(false));
            let (signal, done, forced) = (
                Arc::clone(&requested),
                Arc::clone(&finished),
                Arc::clone(&fallback),
            );
            let writer = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                signal.store(true, Ordering::Release);
                let until = Instant::now() + Duration::from_secs(2);
                while !done.load(Ordering::Acquire) && Instant::now() < until {
                    std::thread::sleep(Duration::from_millis(5));
                }
                forced.store(!done.load(Ordering::Acquire), Ordering::Release);
                drop(output);
            });
            let result = read_confirmation(
                &input,
                LinuxDevelopmentConfirmationKind::OperationApproval,
                &requested,
            );
            finished.store(true, Ordering::Release);
            writer.join().unwrap();
            assert!(
                !fallback.load(Ordering::Acquire),
                "test cleanup cannot establish cancellation"
            );
            assert_eq!(result.unwrap(), LinuxDevelopmentConfirmation::Cancelled);
            assert!(requested.load(Ordering::Acquire));
            assert_eq!(rustix::fs::fcntl_getfl(&input).unwrap(), flags);
        }
    }

    #[test]
    fn write_only_input_is_rejected_before_waiting_for_peer_cleanup() {
        use std::sync::Arc;
        use std::time::{Duration, Instant};
        let (reader, writer) = std::io::pipe().unwrap();
        let finished = Arc::new(AtomicBool::new(false));
        let forced = Arc::new(AtomicBool::new(false));
        let (done, fallback) = (Arc::clone(&finished), Arc::clone(&forced));
        let peer = std::thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(2);
            while !done.load(Ordering::Acquire) && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(5));
            }
            fallback.store(!done.load(Ordering::Acquire), Ordering::Release);
            drop(reader);
        });
        let result = read_confirmation(
            &writer,
            LinuxDevelopmentConfirmationKind::OperationApproval,
            &AtomicBool::new(false),
        );
        finished.store(true, Ordering::Release);
        peer.join().unwrap();
        assert_eq!(
            result.unwrap_err().kind(),
            LinuxDevelopmentBoundaryErrorKind::ConfirmationInputFailed
        );
        assert!(
            !forced.load(Ordering::Acquire),
            "write-only stdin must refuse before the test closes its peer"
        );
    }
}
