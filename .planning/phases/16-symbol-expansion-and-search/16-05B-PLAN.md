---
phase: 16-symbol-expansion-and-search
plan: 05B
type: execute
wave: 3
depends_on: [16-01, 16-02, 16-03, 16-04, 16-05A]
files_modified:
  - tests/expansion_tests.rs
autonomous: true
user_setup: []

must_haves:
  truths:
    - "Expansion works consistently across C/C++, Java, JavaScript, and TypeScript"
    - "Each language has at least 3 test cases covering different symbol types"
    - "Tests verify progressive expansion (level 0, 1, 2)"
    - "Tests verify doc comment inclusion for each language's doc style"
  artifacts:
    - path: "tests/expansion_tests.rs"
      provides: "Expansion tests for C/C++, Java, JavaScript, TypeScript"
      min_lines: 200
  key_links:
    - from: "tests/expansion_tests.rs"
      to: "expand_symbol()"
      via: "direct function calls"
      pattern: "splice::expand::expand_symbol"
---

<objective>
Test expansion across C/C++, Java, JavaScript, and TypeScript for consistency.

Purpose: Ensure symbol expansion works correctly across the remaining supported languages (C, C++, Java, JavaScript, TypeScript) with comprehensive test coverage for different symbol types and expansion levels.

Output: Extended integration test suite in tests/expansion_tests.rs with 15 additional tests (3 per language) covering functions, classes/structs, and nested symbols, plus 4 progressive expansion level tests.
</objective>

<execution_context>
@~/.claude/get-shit-done/workflows/execute-plan.md
@~/.claude/get-shit-done/templates/summary.md

@.planning/phases/16-symbol-expansion-and-search/16-RESEARCH.md

@src/expand/mod.rs
@src/expand/tree_walker.rs
@tests/context_flags_tests.rs
</context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md

@.planning/phases/16-symbol-expansion-and-search/16-01-PLAN.md
@.planning/phases/16-symbol-expansion-and-search/16-03-PLAN.md
@.planning/phases/16-symbol-expansion-and-search/16-04-PLAN.md
@.planning/phases/16-symbol-expansion-and-search/16-05A-PLAN.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add C/C++ expansion tests (6 tests)</name>
  <files>tests/expansion_tests.rs</files>
  <action>
Add 3 C-specific tests:
1. test_c_function_expansion - expands function with signature and body
2. test_c_struct_expansion - expands struct definition
3. test_c_expansion_with_comments - verifies /* */ and /// comment capture

Add 3 C++-specific tests:
1. test_cpp_class_expansion - expands class definition with members
2. test_cpp_method_expansion - expands method with signature
3. test_cpp_method_to_class_expansion - level 2 expands method to class
  </action>
  <verify>cargo test --quiet expansion_tests::cpp</verify>
  <done>All 6 C/C++ expansion tests pass</done>
</task>

<task type="auto">
  <name>Task 2: Add Java expansion tests (3 tests)</name>
  <files>tests/expansion_tests.rs</files>
  <action>
Add 3 Java-specific tests:
1. test_java_method_expansion - expands method with signature and body
2. test_java_class_expansion - expands class definition with fields and methods
3. test_java_method_to_class_expansion - level 2 expands method to class

Include Javadoc (/** */) in test fixtures.
  </action>
  <verify>cargo test --quiet expansion_tests::java</verify>
  <done>All 3 Java expansion tests pass</done>
</task>

<task type="auto">
  <name>Task 3: Add JavaScript/TypeScript expansion tests (6 tests)</name>
  <files>tests/expansion_tests.rs</files>
  <action>
Add 3 JavaScript-specific tests:
1. test_js_function_expansion - expands function declaration
2. test_js_class_expansion - expands class definition
3. test_js_method_to_class_expansion - level 2 expands method to class

Add 3 TypeScript-specific tests:
1. test_ts_function_expansion - expands function with type annotations
2. test_ts_interface_expansion - expands interface definition
3. test_ts_method_to_interface_expansion - level 2 expands method to interface/class

Include JSDoc (/** */) in test fixtures.
  </action>
  <verify>cargo test --quiet expansion_tests::js</verify>
  <done>All 6 JavaScript/TypeScript expansion tests pass</done>
</task>

<task type="auto">
  <name>Task 4: Add progressive expansion level tests</name>
  <files>tests/expansion_tests.rs</files>
  <action>
Add cross-language tests for expansion levels:
1. test_level_0_no_expansion - all languages return original span
2. test_level_1_body_expansion - all languages return symbol body
3. test_level_2_containing_block - nested symbols expand to containing block
4. test_level_2_no_containing_block - top-level symbols stay at level 1

These tests verify the progressive expansion behavior from plan 16-03 across all 7 languages.
  </action>
  <verify>cargo test --quiet expansion_tests::progressive</verify>
  <done>All 4 progressive expansion tests pass</done>
</task>

</tasks>

<verification>
- [ ] `cargo check` passes with no errors
- [ ] `cargo test --quiet expansion_tests` passes all tests
- [ ] All 7 languages have at least 3 tests
- [ ] Tests cover expansion levels 0, 1, 2
- [ ] Tests verify doc comment inclusion for each language's doc style
- [ ] Total test count >= 21 (3 per language minimum + 4 progressive)
</verification>

<success_criteria>
1. tests/expansion_tests.rs extended to 21+ tests
2. C/C++ has 6 tests, Java has 3 tests, JS/TS has 6 tests
3. Progressive expansion levels (0, 1, 2) verified across all languages
4. Doc comment inclusion verified for each language's doc style (///, /**/, """, /**, JSDoc)
5. All tests pass
</success_criteria>

<output>
After completion, create `.planning/phases/16-symbol-expansion-and-search/16-05B-SUMMARY.md`
</output>
