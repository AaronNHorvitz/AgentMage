"use strict";
const $ = (id) => document.getElementById(id);
let token = location.hash.slice(1);
history.replaceState(null, "", location.pathname);
let active = null;
async function api(op, data = {}) {
  const r = await fetch("/api/" + op, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-AgentMage-Token": token },
    body: JSON.stringify(data),
  });
  const v = await r.json();
  if (!r.ok || !v.ok) throw Error(v.error || "Request failed");
  return v;
}
function status(s, error = false) {
  $("status").textContent = s;
  $("status").className = error ? "error" : "";
}
function busy(v) {
  $("send").disabled = v;
  $("cancel").disabled = !v;
  $("load").disabled = v;
  $("browse").disabled = v;
  $("new").disabled = v;
}
function message(role, text) {
  const n = document.createElement("div");
  n.className = "message " + role;
  const h = document.createElement("strong");
  h.textContent = role === "user" ? "YOU" : "AGENTMAGE · LOCAL MODEL";
  n.append(h, document.createTextNode(text));
  $("messages").append(n);
  n.scrollIntoView({ block: "nearest" });
  return n;
}
function citations(n, items) {
  for (const c of items) {
    const d = document.createElement("details");
    d.className = "citation";
    const s = document.createElement("summary");
    s.textContent =
      "[" +
      c.id +
      "] " +
      c.path +
      " · lines " +
      c.start_line +
      "–" +
      c.end_line;
    const p = document.createElement("pre");
    p.textContent = c.text;
    const m = document.createElement("div");
    m.className = "meta";
    m.textContent = "Read-only source snapshot · SHA-256 " + c.content_sha256;
    d.append(s, p, m);
    n.append(d);
  }
}
async function refresh() {
  const v = await api("status");
  $("modelState").textContent = v.model_ready
    ? "● Local model ready"
    : v.model_starting
      ? "Model loading…"
      : "Model unavailable";
  $("modelDetail").textContent = v.model_label + " · " + v.model_state;
  $("turns").textContent = v.turns + " / 6 turns";
  if (!$("folder").value) $("folder").value = v.fixture;
  return v;
}
async function load() {
  busy(true);
  try {
    const v = await api("ingest", { folder: $("folder").value });
    $("files").replaceChildren();
    for (const f of v.files) {
      const li = document.createElement("li");
      li.textContent = f.path + " · " + f.status;
      const s = document.createElement("small");
      s.textContent = f.reason;
      li.append(s);
      $("files").append(li);
    }
    $("folderStatus").textContent =
      v.accepted_count +
      " files accepted. " +
      (v.complete === false ? "Inventory incomplete; narrow the folder." : "");
    $("messages").replaceChildren();
    status("Folder loaded. Ask a question.");
    await refresh();
  } catch (e) {
    status(e.message, true);
  } finally {
    busy(false);
  }
}
$("load").onclick = load;
$("fixture").onclick = async () => {
  try {
    const v = await refresh();
    $("folder").value = v.fixture;
    await load();
  } catch (e) {
    status(e.message, true);
  }
};
$("browse").onclick = async () => {
  try {
    const v = await api("browse");
    $("folder").value = v.folder;
    await load();
  } catch (e) {
    status(e.message, true);
  }
};
$("example").onclick = () => {
  $("question").value = "When does Aurora launch, and who leads it?";
  $("question").focus();
};
$("new").onclick = async () => {
  try {
    await api("new");
    $("messages").replaceChildren();
    status("New conversation ready.");
    await refresh();
  } catch (e) {
    status(e.message, true);
  }
};
$("ask").onsubmit = async (e) => {
  e.preventDefault();
  const question = $("question").value.trim();
  if (!question) return;
  busy(true);
  $("cancel").disabled = true;
  message("user", question);
  status("Retrieving source evidence…");
  try {
    const v = await api("ask", { question });
    active = v.job;
    $("cancel").disabled = false;
    status("Generating locally…");
    while (active) {
      await new Promise((r) => setTimeout(r, 250));
      const p = await api("result", { job: active });
      if (!p.done) continue;
      active = null;
      if (p.error) throw Error(p.error);
      const n = message("assistant", p.answer);
      citations(n, p.citations);
      const m = document.createElement("p");
      m.className = "meta";
      m.textContent =
        p.elapsed_seconds +
        " s · " +
        p.prompt_tokens +
        " prompt tokens · " +
        p.output_tokens +
        " output tokens" +
        (p.omitted_count
          ? " · " +
            p.omitted_count +
            " retrieved fragments omitted; evidence is partial"
          : "");
      n.append(m);
      $("question").value = "";
      status(
        p.insufficient_evidence
          ? "Insufficient evidence in the selected source content."
          : "Answer complete. Inspect the source citations above.",
      );
      await refresh();
    }
  } catch (e) {
    status(e.message, true);
  } finally {
    active = null;
    busy(false);
  }
};
$("cancel").onclick = async () => {
  try {
    await api("cancel");
    status("Cancellation requested…");
  } catch (e) {
    status(e.message, true);
  }
};
$("modelStop").onclick = async () => {
  try {
    await api("model-stop");
    await refresh();
    status("Model stopped. Use Start / retry model to recover.");
  } catch (e) {
    status(e.message, true);
  }
};
$("modelStart").onclick = async () => {
  try {
    await api("model-start");
    status("Starting the local model…");
    await refresh();
  } catch (e) {
    status(e.message, true);
  }
};
refresh().catch((e) =>
  status(e.message + " Open the launch URL containing the access token.", true),
);
setInterval(() => refresh().catch(() => {}), 3000);
