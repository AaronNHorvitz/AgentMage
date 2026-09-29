from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.rust_test_ownership import (
    AmbiguousReference,
    production_sources,
    test_only_files,
)


ROOT = Path(__file__).resolve().parents[1]
SRC = Path("crate/src")
EXTERNAL = '#[cfg(test)]\n#[path = "{name}"]\nmod tests;\n'


def owned(files: dict[str, str]) -> set[str]:
    sources = {SRC / name: text for name, text in files.items()}
    return {str(path.relative_to(SRC)) for path in test_only_files(sources)}


class RustTestOwnershipTests(unittest.TestCase):
    def test_exact_external_test_module_and_its_transitive_files_are_test_only(self) -> None:
        files = {
            "lib.rs": "pub mod engine;\n#[cfg(test)]\nmod fixtures;\n",
            "engine.rs": "pub fn run() {}\n" + EXTERNAL.format(name="engine_tests.rs"),
            "engine_tests.rs": '#[path = "engine_more_tests.rs"]\nmod more;\n'
            'mod nested {\n    include!("engine_included_tests.rs");\n}\n',
            "engine_more_tests.rs": "fn muse() {}\n",
            "engine_included_tests.rs": "fn gemma() {}\n",
            "fixtures.rs": "fn fixture() {}\n",
        }
        self.assertEqual(
            owned(files),
            {"engine_tests.rs", "engine_more_tests.rs", "engine_included_tests.rs", "fixtures.rs"},
        )

    def test_include_inside_an_inline_test_module_is_test_only(self) -> None:
        files = {
            "lib.rs": "pub mod store;\n",
            "store.rs": 'fn keep() {}\n#[cfg(test)]\nmod tests {\n    mod a {\n'
            '        include!("store_cases.rs");\n    }\n}\n',
            "store_cases.rs": "fn case() {}\n",
        }
        self.assertEqual(owned(files), {"store_cases.rs"})

    def test_any_production_reference_keeps_a_file_in_production(self) -> None:
        files = {
            "lib.rs": "pub mod engine;\n",
            "engine.rs": EXTERNAL.format(name="shared.rs")
            + '#[path = "shared.rs"]\nmod shared_production;\n',
            "shared.rs": "fn muse() {}\n",
        }
        self.assertEqual(owned(files), set())
        files["engine.rs"] = EXTERNAL.format(name="shared.rs") + 'include!("shared.rs");\n'
        self.assertEqual(owned(files), set())

    def test_restricted_visibility_or_unrecognized_attributes_stay_production(self) -> None:
        sources = {
            SRC / "lib.rs": "pub mod engine;\n#[cfg(test)]\nmod helpers;\n",
            SRC / "engine.rs": "pub(in crate) mod restricted;\n",
            SRC / "engine/restricted.rs": "fn production() {}\n",
            SRC / "helpers.rs": '#[path = "engine/restricted.rs"]\nmod alias;\n',
        }
        with self.assertRaises(AmbiguousReference):
            test_only_files(sources)
        sources[SRC / "helpers.rs"] = "fn helper() {}\n"
        sources[SRC / "engine.rs"] = "#[cfg(test)]\npub(in crate) mod restricted;\n"
        # The cfg(test) attribute is not recognized across this visibility form,
        # so the module remains a production reference.
        self.assertEqual(test_only_files(sources), {SRC / "helpers.rs"})

    def test_default_module_paths_follow_mod_rs_and_non_mod_rs_rules(self) -> None:
        sources = {
            SRC / "lib.rs": "pub mod loop_owner;\n#[cfg(test)]\nmod helpers;\n",
            SRC / "loop_owner.rs": "mod stage;\n#[cfg(test)]\nmod cases;\n",
            SRC / "loop_owner/stage.rs": "fn production() {}\n",
            SRC / "loop_owner/cases.rs": "fn test_case() {}\n",
            SRC / "helpers.rs": "fn helper() {}\n",
            # Same file name as the nested test module, but never declared at this path.
            SRC / "cases.rs": "fn unrelated() {}\n",
        }
        self.assertEqual(
            test_only_files(sources),
            {SRC / "loop_owner/cases.rs", SRC / "helpers.rs"},
        )

    def test_comments_strings_other_cfgs_and_unreferenced_files_never_exempt(self) -> None:
        files = {
            "lib.rs": "pub mod engine;\n",
            "engine.rs": "// " + EXTERNAL.format(name="commented.rs").replace("\n", "\n// ")
            + '\nconst TEXT: &str = "#[cfg(test)] #[path = \\"quoted.rs\\"] mod q;";\n'
            + '#[cfg(any(test, feature = "fixture"))]\n#[path = "feature.rs"]\nmod feature;\n'
            + '#[cfg(all(test, unix))]\n#[path = "compound.rs"]\nmod compound;\n',
            "commented.rs": "fn muse() {}\n",
            "quoted.rs": "fn muse() {}\n",
            "feature.rs": "fn muse() {}\n",
            "compound.rs": "fn muse() {}\n",
            "orphan.rs": "fn muse() {}\n",
        }
        self.assertEqual(owned(files), set())

    def test_unresolvable_references_are_ambiguous(self) -> None:
        for engine in (
            '#[cfg(test)]\n#[path = "../outside.rs"]\nmod tests;\n',
            '#[cfg(test)]\n#[path = "nested/tests.rs"]\nmod tests;\n',
            'pub mod inner {\n    #[path = "inner_impl.rs"]\n    mod implementation;\n}\n',
            "pub mod inner {\n    mod implementation;\n}\n",
            'include!(concat!("gen", ".rs"));\n',
            'include!("../generated.rs");\n',
            "mod\nsplit;\n",
            '#[path = "stray.rs"]\nfn not_a_module() {}\n',
            '#[cfg(test)]\n#[path = "a.rs"]\n#[path = "b.rs"]\nmod tests;\n',
            'const OPEN: &str = "unterminated;\n',
            "#[cfg(test)]\nmod tests {\n    fn unbalanced() {\n}\n",
        ):
            with self.subTest(engine=engine):
                with self.assertRaises(AmbiguousReference):
                    test_only_files({SRC / "lib.rs": "pub mod engine;\n", SRC / "engine.rs": engine})

    def test_forms_that_can_select_a_production_file_are_ambiguous(self) -> None:
        test_reference = EXTERNAL.format(name="shared.rs")
        for engine in (
            # A cfg_attr path selects the file for non-test builds.
            '#[cfg_attr(not(test), path = "shared.rs")]\nmod real;\n' + test_reference,
            '#[cfg_attr(\n    not(test),\n    path = "shared.rs"\n)]\nmod real;\n' + test_reference,
            '#![cfg_attr(feature = "x", path = "shared.rs")]\n' + test_reference,
            # A macro body can declare a file module from a fragment or a literal name.
            "macro_rules! declare {\n    ($name:ident) => {\n        mod $name;\n    };\n}\n"
            "declare!(shared);\n" + test_reference,
            "macro_rules! declare {\n    () => { mod shared; };\n}\ndeclare!();\n" + test_reference,
        ):
            with self.subTest(engine=engine):
                with self.assertRaises(AmbiguousReference):
                    owned({"lib.rs": "pub mod engine;\n", "engine.rs": engine, "shared.rs": "fn muse() {}\n"})

    def test_raw_identifier_declarations_are_production_references(self) -> None:
        files = {
            "lib.rs": "mod r#gen;\n" + EXTERNAL.format(name="gen.rs"),
            "gen.rs": "fn muse() {}\n",
        }
        self.assertEqual(owned(files), set())
        files["lib.rs"] = "#[cfg(test)]\nmod r#gen;\n"
        self.assertEqual(owned(files), {"gen.rs"})

    def test_children_of_path_loaded_and_included_files_resolve_beside_them(self) -> None:
        # The compiler resolves `mod q;` in a `#[path]`-loaded or included file
        # next to that file, so `q.rs` is production and never exempted.
        for loader in ('#[path = "p.rs"]\nmod p;\n', 'include!("p.rs");\n'):
            for owner in ("lib.rs", "engine.rs"):
                with self.subTest(loader=loader, owner=owner):
                    files = {
                        "p.rs": "mod q;\n",
                        "q.rs": "fn muse() {}\n",
                        "engine/q.rs": "fn unrelated() {}\n",
                        "p/q.rs": "fn unrelated() {}\n",
                    }
                    if owner == "lib.rs":
                        files["lib.rs"] = loader + EXTERNAL.format(name="q.rs")
                    else:
                        files["lib.rs"] = "pub mod engine;\n"
                        files["engine.rs"] = loader + EXTERNAL.format(name="q.rs")
                    self.assertEqual(owned(files), set())
        # A default-loaded non-mod-rs file keeps its child directory.
        files = {
            "lib.rs": "pub mod p;\n" + EXTERNAL.format(name="q.rs"),
            "p.rs": "mod q;\n",
            "p/q.rs": "fn production() {}\n",
            "q.rs": "fn test_only() {}\n",
        }
        self.assertEqual(owned(files), {"q.rs"})

    def test_a_file_whose_children_depend_on_its_loader_is_ambiguous(self) -> None:
        files = {
            "lib.rs": 'pub mod p;\n#[path = "p.rs"]\nmod p_again;\n',
            "p.rs": "mod q;\n",
            "q.rs": "fn muse() {}\n",
            "p/q.rs": "fn muse() {}\n",
        }
        with self.assertRaises(AmbiguousReference):
            owned(files)
        files["p.rs"] = "fn no_children() {}\n"
        self.assertEqual(owned(files), set())

    def test_ambiguity_anywhere_disables_every_whole_file_exemption(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            src = Path(directory)
            (src / "lib.rs").write_text("pub mod engine;\npub mod other;\n", encoding="utf-8")
            (src / "engine.rs").write_text(EXTERNAL.format(name="engine_tests.rs"), encoding="utf-8")
            (src / "engine_tests.rs").write_text("fn muse() {}\n", encoding="utf-8")
            (src / "other.rs").write_text('#[path = "../elsewhere.rs"]\nmod x;\n', encoding="utf-8")
            production = production_sources(sorted(src.glob("*.rs")))
            self.assertEqual(production[src / "engine_tests.rs"], "fn muse() {}\n")
            (src / "other.rs").write_text("pub fn fine() {}\n", encoding="utf-8")
            production = production_sources(sorted(src.glob("*.rs")))
            self.assertEqual(production[src / "engine_tests.rs"], "")

    def test_inline_test_modules_are_blanked_but_trailing_production_is_kept(self) -> None:
        source = (
            "fn before() {}\n"
            "#[cfg(test)]\npub(crate) mod tests_support {\n    fn muse() {}\n}\n"
            "#[cfg(test)]\nmod tests {\n    fn gemma() { let s = \"}\"; }\n}\n"
            "fn after_muse() {}\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lib.rs"
            path.write_text(source, encoding="utf-8")
            production = production_sources([path])[path]
        self.assertIn("fn before() {}", production)
        self.assertIn("fn after_muse() {}", production)
        self.assertNotIn("fn muse()", production)
        self.assertNotIn("fn gemma()", production)
        self.assertEqual(production.count("\n"), source.count("\n"))
        self.assertEqual(
            production.splitlines().index("fn after_muse() {}"),
            source.splitlines().index("fn after_muse() {}"),
        )

    def test_current_kernel_ownership_is_exact(self) -> None:
        paths = sorted(ROOT.glob("kernel/*/src/**/*.rs"))
        exempt = test_only_files({path: path.read_text(encoding="utf-8") for path in paths})
        engine = ROOT / "kernel/engine/src"
        self.assertIn(engine / "runtime_loop_tests.rs", exempt)
        self.assertIn(engine / "runtime_loop_artifact_preparation_tests.rs", exempt)
        self.assertIn(engine / "research_journal_tests.rs", exempt)
        self.assertNotIn(engine / "runtime_loop/artifact_preparation.rs", exempt)
        self.assertNotIn(engine / "research_report_retained.rs", exempt)
        self.assertNotIn(engine / "frontier_import_recovery.rs", exempt)
        self.assertNotIn(engine / "runtime_loop.rs", exempt)
        self.assertNotIn(engine / "lib.rs", exempt)
        self.assertTrue(all(path.name.endswith(".rs") for path in exempt))


if __name__ == "__main__":
    unittest.main()
