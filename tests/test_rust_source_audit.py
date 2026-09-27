from __future__ import annotations

import ast
import unittest
from pathlib import Path

from scripts.rust_source_audit import production_source


class RustSourceAuditTests(unittest.TestCase):
    def test_explicit_evidence_bindings_include_the_extracted_helper_and_its_tests(self):
        scripts = Path(__file__).resolve().parents[1] / "scripts"
        audited = {"scripts/effect_boundary.py", "scripts/strict_local_source_audit.py"}
        required = {"scripts/rust_source_audit.py", "tests/test_rust_source_audit.py"}
        matched = []
        for path in sorted(scripts.glob("*.py")):
            for node in ast.parse(path.read_text(encoding="utf-8")).body:
                if not (isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name)
                        and node.target.id in {"SOURCE_PATHS", "EVIDENCE_PATHS"}):
                    continue
                strings = {item.value for item in ast.walk(node.value)
                           if isinstance(item, ast.Constant) and isinstance(item.value, str)}
                if audited & strings:
                    self.assertTrue(required <= strings, path.name)
                    matched.append(path.name)
        self.assertGreaterEqual(len(matched), 11)

    def test_bounded_cache_keys_complete_source_not_path_or_metadata(self):
        production_source.cache_clear()
        source = "#[cfg(test)]\nmod tests {}\nfn original() {}"
        self.assertIn("fn original", production_source(source))
        self.assertIn("fn original", production_source(source))
        self.assertEqual(production_source.cache_info().hits, 1)
        changed = source + "\nimpl EffectDriver for Bypass {}"
        self.assertIn("impl EffectDriver for Bypass", production_source(changed))
        self.assertEqual(production_source.cache_info().misses, 2)
        self.assertEqual(production_source.cache_info().maxsize, 1024)

    def test_trailing_production_is_never_a_test_exemption(self):
        source = "fn before() {}\n#[cfg(test)]\nmod tests { fn fake() {} }\nimpl EffectDriver for Bypass {}\n"
        observed = production_source(source)
        self.assertIn("fn before() {}", observed)
        self.assertIn("impl EffectDriver for Bypass {}", observed)
        self.assertNotIn("fn fake", observed)
        self.assertEqual(source.count("\n"), observed.count("\n"))

    def test_nested_braces_literals_comments_and_lifetimes_do_not_change_boundary(self):
        source = r'''fn before() {}
#[cfg(test)]
mod tests {
    fn fake<'a>(value: &'a str) {
        let s = "escaped \" } quote";
        let raw = br###" } " ## { "###;
        let ch = '}'; let escaped = '\''; let unicode = '\u{7d}';
        /* } /* nested { */ { */
        // }
        if true { () }
    }
}
fn outside() { forbidden(); }
'''
        observed = production_source(source)
        self.assertNotIn("fn fake", observed)
        self.assertIn("fn outside() { forbidden(); }", observed)

    def test_test_marker_inside_string_or_comment_is_not_an_exemption(self):
        for prefix, suffix in [('r###"', '"###;'), ('/*', '*/')]:
            source = prefix + "\n#[cfg(test)]\nmod tests {\n" + suffix + "\nfn outside() {}\n"
            self.assertEqual(production_source(source), source)

    def test_multiple_modules_keep_intervening_and_later_items(self):
        source = "#[cfg(test)]\nmod tests {}\nfn middle() {}\n#[cfg(test)]\nmod tests {}\nfn end() {}"
        observed = production_source(source)
        self.assertIn("fn middle() {}", observed)
        self.assertIn("fn end() {}", observed)
        self.assertNotIn("mod tests", observed)

    def test_separate_platform_test_module_does_not_hide_later_product_code(self):
        source = '''#[cfg(test)]
mod tests {}
fn between() {}
#[cfg(all(test, target_os = "linux"))]
mod linux_tests { fn native_fixture() {} }
fn after() { forbidden(); }
'''
        observed = production_source(source)
        self.assertNotIn("native_fixture", observed)
        self.assertIn("fn between() {}", observed)
        self.assertIn("fn after() { forbidden(); }", observed)

    def test_malformed_input_is_kept_in_full(self):
        for source in [
            "#[cfg(test)]\nmod tests { fn outside() {}",
            '#[cfg(test)]\nmod tests {}\nlet bad = "unterminated',
            '#[cfg(test)]\nmod tests {}\nlet bad = r#"unterminated',
            "#[cfg(test)]\nmod tests {}\n/* unterminated",
        ]:
            self.assertEqual(production_source(source), source)

    def test_unknown_or_negated_configuration_is_not_exempted(self):
        for attribute in ["cfg(not(test))", 'cfg(feature = "test")', "cfg(any(test, unix))"]:
            source = f"#[{attribute}]\nmod tests {{ fn effect() {{}} }}"
            self.assertEqual(production_source(source), source)
