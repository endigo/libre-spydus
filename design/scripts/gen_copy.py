#!/usr/bin/env python3
"""Generate the apps' Copy files from design/copy.json (docs/native-apps.md §7.1).

    design/scripts/gen_copy.py           write the three files
    design/scripts/gen_copy.py --check   exit 1 when any file is out of date (writes nothing)

Writes ios/LibreSpydus/Core/Copy.swift (`enum Copy { enum Picks { static let title = … } }`),
android/app/src/main/java/com/librespydus/app/core/Copy.kt (`object Copy { object Picks { const
val title = … } }`) and desktop/src/copy.rs (`pub mod Copy { pub mod Picks { pub const title:
&str = …; } }`, docs/desktop-app.md §8.1), with the same names on every platform:

- each JSON object becomes a nested enum / object (`yourBooks` → `YourBooks`), each leaf a member
  with the leaf's camelCase key;
- a string without placeholders is a constant; one with {named} placeholders is a function whose
  labelled parameters follow the order of first appearance (`count: Int`, every other one
  `String`; Rust `count: i64` and `&str`); {count} is shown with the locale's digit grouping
  ("1,234 results"; Rust always groups with commas);
- {"one": …, "other": …} is a function of `count: Int` (first) plus the other placeholders,
  choosing "one" when count == 1;
- {"ios": …, "android": …, "desktop": …} (a platform variant, docs/desktop-app.md §8.2) resolves to
  that platform's value. It lists every platform; each value is a string or a {"one", "other"}
  plural, and all of them have the same kind and the same placeholders. The other platforms'
  words become doc comments. It may add "ipad" (docs/native-apps.md §6.1): the iOS app's words on
  an iPad, chosen at run time (`CopyDevice.usesPadWords`) where an iPhone gets "ios"; and "tablet":
  the Android app's words on a tablet, chosen at run time (`CopyDevice.usesTabletWords`) where a
  phone gets "android";
- identifiers that are keywords are escaped (Swift `use`, Kotlin `when`, Rust `r#use`);
- keys starting with "_" are notes: "_notes" entries become doc comments, the rest are skipped.

Python 3.9+ (the Mac mini's /usr/bin/python3): no `match`, no `X | Y` outside annotations.

Python 3 standard library only; no network.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "design" / "copy.json"
SWIFT_OUT = ROOT / "ios" / "LibreSpydus" / "Core" / "Copy.swift"
KOTLIN_OUT = ROOT / "android" / "app" / "src" / "main" / "java" / "com" / "librespydus" / "app" / "core" / "Copy.kt"
RUST_OUT = ROOT / "desktop" / "src" / "copy.rs"

PLATFORMS = ("ios", "android", "desktop")
PLATFORM_NAMES = {"ios": "iOS", "android": "Android", "desktop": "Desktop"}
# Optional in a platform variant: the iOS app's words on an iPad, the Android app's on a tablet.
IPAD = "ipad"
TABLET = "tablet"
DEVICE_FORMS = (IPAD, TABLET)
VARIANT_KEYS = set(PLATFORMS) | set(DEVICE_FORMS)

HEADER = "GENERATED from design/copy.json by design/scripts/gen_copy.py — do not edit"

KEY = re.compile(r"^[a-z][A-Za-z0-9]*$")
PLACEHOLDER = re.compile(r"\{([a-z][A-Za-z0-9]*)\}")

SWIFT_KEYWORDS = {
    "associatedtype", "class", "deinit", "enum", "extension", "fileprivate", "func", "import", "init",
    "inout", "internal", "let", "open", "operator", "private", "precedencegroup", "protocol", "public",
    "rethrows", "static", "struct", "subscript", "typealias", "var", "break", "case", "catch",
    "continue", "default", "defer", "do", "else", "fallthrough", "for", "guard", "if", "in", "repeat",
    "return", "throw", "switch", "where", "while", "as", "false", "is", "nil", "self", "Self",
    "super", "throws", "true", "try", "Type", "Protocol",
}
KOTLIN_KEYWORDS = {
    "as", "break", "class", "continue", "do", "else", "false", "for", "fun", "if", "in", "interface",
    "is", "null", "object", "package", "return", "super", "this", "throw", "true", "try", "typealias",
    "typeof", "val", "var", "when", "while",
}


RUST_KEYWORDS = {
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if",
    "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "async",
    "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof",
    "unsized", "virtual", "yield", "try", "gen",
}
# Keywords a raw identifier can't spell (`r#self` is an error): suffixed with "_" instead.
RUST_NOT_RAW = {"self", "Self", "super", "crate"}


class CopyError(Exception):
    pass


# ---- model ------------------------------------------------------------------------------------

class Leaf:
    """One string: plain, plural or a platform variant, with its value on each platform.

    `forms[platform]` is {"text": …} or {"one": …, "other": …}; `plural` says which (the same on
    every platform). `ipad` is the iPad's form of a variant that has one, else None; `tablet` the
    Android tablet's.
    """

    def __init__(self, path: list[str], value, note: str | None):
        self.path = path
        self.name = path[-1]
        self.note = note
        self.raw = value
        where = ".".join(path)
        self.ipad = None
        self.tablet = None
        if isinstance(value, dict) and set(PLATFORMS) <= set(value) <= VARIANT_KEYS:
            self.kind = "variant"
            keys = PLATFORMS + tuple(form for form in DEVICE_FORMS if form in value)
            forms = {key: _forms(f"{where}.{key}", value[key]) for key in keys}
            kinds = {platform: "one" in form for platform, form in forms.items()}
            if len(set(kinds.values())) != 1:
                raise CopyError(f"{where}: every platform's value must be a string, or every one a plural")
            names = {platform: _placeholder_set(form) for platform, form in forms.items()}
            if len({frozenset(n) for n in names.values()}) != 1:
                raise CopyError(f"{where}: every platform's value must use the same placeholders")
            self.plural = kinds["ios"]
            self.ipad = forms.pop(IPAD, None)
            self.tablet = forms.pop(TABLET, None)
            self.forms = forms
        elif isinstance(value, dict) and (set(value) & set(PLATFORMS)) and set(value) <= VARIANT_KEYS:
            missing = ", ".join(p for p in PLATFORMS if p not in value)
            raise CopyError(f"{where}: a platform variant lists every platform (missing {missing})")
        elif isinstance(value, str) or (isinstance(value, dict) and set(value) == {"one", "other"}):
            form = _forms(where, value)
            self.kind = "plural" if "one" in form else "text"
            self.plural = self.kind == "plural"
            self.forms = {platform: form for platform in PLATFORMS}
        else:
            raise CopyError(f"{where}: a leaf is a string, {{\"one\", \"other\"}} or "
                            f"{{\"ios\", \"android\", \"desktop\"}} (+ \"ipad\", \"tablet\")")
        for forms in list(self.forms.values()) + [form for form in (self.ipad, self.tablet) if form]:
            for text in forms.values():
                if not text.strip():
                    raise CopyError(f"{where}: empty text")
                if any(brace in PLACEHOLDER.sub("", text) for brace in "{}"):
                    raise CopyError(f"{where}: stray brace in {text!r}")

    # The Swift and Kotlin emitters predate variants and read `texts`.
    @property
    def texts(self) -> dict:
        return self.forms

    def params(self, platform: str) -> list[str]:
        """Placeholders in order of first appearance; a plural's count first."""
        forms = self.forms[platform]
        names: list[str] = ["count"] if self.plural else []
        for key in ("other", "one") if self.plural else ("text",):
            for name in placeholders(forms[key]):
                if name not in names:
                    names.append(name)
        return names

    def describe(self, platform: str) -> str:
        """This platform's value for a doc comment: `"…"`, or `One: "…" · other: "…"`."""
        forms = self.forms[platform]
        if self.plural:
            return f'One: "{forms["one"]}" · other: "{forms["other"]}"'
        return f'"{forms["text"]}"'


def _describe_forms(forms: dict, plural: bool) -> str:
    """A device form for a doc comment: `"…"`, or `One: "…" · other: "…"`."""
    if plural:
        return f'One: "{forms["one"]}" · other: "{forms["other"]}"'
    return f'"{forms["text"]}"'


def _forms(where: str, value) -> dict:
    if isinstance(value, str):
        return {"text": value}
    if isinstance(value, dict) and set(value) == {"one", "other"}:
        forms = {"one": value["one"], "other": value["other"]}
        if not all(isinstance(text, str) for text in forms.values()):
            raise CopyError(f"{where}: plural forms must be strings")
        if "count" not in placeholders(forms["other"]):
            raise CopyError(f"{where}: a plural's \"other\" form must use {{count}}")
        return forms
    raise CopyError(f"{where}: a value is a string or {{\"one\", \"other\"}}")


def _placeholder_set(forms: dict) -> set:
    names = set()
    for text in forms.values():
        names.update(placeholders(text))
    return names


class Group:
    def __init__(self, path: list[str], obj: dict):
        self.path = path
        self.name = path[-1] if path else "copy"
        self.children: list[Group | Leaf] = []
        notes = obj.get("_notes", {})
        if not isinstance(notes, dict):
            raise CopyError(f"{'.'.join(path) or '(root)'}._notes must be an object")
        for key in notes:
            if key not in obj:
                raise CopyError(f"{'.'.join(path + ['_notes', key])}: no such key")
        for key, value in obj.items():
            if key.startswith("_"):
                continue
            if not KEY.match(key):
                raise CopyError(f"{'.'.join(path + [key])}: keys are camelCase")
            child_path = path + [key]
            if isinstance(value, dict) and not _is_leaf_dict(value):
                if key in notes:
                    raise CopyError(f"{'.'.join(path + ['_notes', key])}: notes go on leaves")
                self.children.append(Group(child_path, value))
            else:
                self.children.append(Leaf(child_path, value, notes.get(key)))
        if not self.children:
            raise CopyError(f"{'.'.join(path) or '(root)'}: empty group")

    def leaves(self):
        for child in self.children:
            if isinstance(child, Group):
                yield from child.leaves()
            else:
                yield child


def _is_leaf_dict(value: dict) -> bool:
    keys = set(value)
    return keys == {"one", "other"} or bool(keys & set(PLATFORMS)) and keys <= VARIANT_KEYS


def placeholders(text: str) -> list[str]:
    seen: list[str] = []
    for name in PLACEHOLDER.findall(text):
        if name not in seen:
            seen.append(name)
    return seen


def type_name(key: str) -> str:
    return key[0].upper() + key[1:]


def load(source: Path = SOURCE) -> Group:
    try:
        data = json.loads(source.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise CopyError(f"{source}: {error}") from error
    if not isinstance(data, dict):
        raise CopyError(f"{source}: the top level must be an object")
    for key, value in data.items():
        if not key.startswith("_") and not isinstance(value, dict):
            raise CopyError(f"{key}: top-level entries are groups")
    return Group([], data)


# ---- Swift ------------------------------------------------------------------------------------

def swift_ident(name: str) -> str:
    return f"`{name}`" if name in SWIFT_KEYWORDS else name


def swift_literal(text: str, params: list[str]) -> str:
    out = []
    pos = 0
    for match in PLACEHOLDER.finditer(text):
        out.append(_swift_escape(text[pos:match.start()]))
        name = match.group(1)
        out.append(f"\\(copyNumber({swift_ident(name)}))" if name == "count" else f"\\({swift_ident(name)})")
        pos = match.end()
    out.append(_swift_escape(text[pos:]))
    return '"' + "".join(out) + '"'


def _swift_escape(text: str) -> str:
    return (text.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
            .replace("\r", "\\r").replace("\t", "\\t"))


def swift_doc(leaf: Leaf, indent: str, platform: str = "ios") -> list[str]:
    lines = []
    if leaf.plural or leaf.params(platform):
        lines.append(f"{indent}/// {leaf.describe(platform)}")
    if leaf.kind == "variant":
        for other in ("android", "desktop"):
            lines.append(f"{indent}/// {PLATFORM_NAMES[other]}: {leaf.describe(other)}")
    if leaf.note:
        lines.append(f"{indent}/// {leaf.note}")
    return lines


def swift_member(leaf: Leaf, indent: str) -> list[str]:
    params = leaf.params("ios")
    forms = leaf.texts["ios"]
    lines = swift_doc(leaf, indent)
    name = swift_ident(leaf.name)
    if not params and not leaf.ipad:
        lines.append(f"{indent}static let {name} = {swift_literal(forms['text'], params)}")
        return lines
    body = swift_body(forms, params, leaf.plural)
    if leaf.ipad:
        # An iPad's words where an iPhone's differ, chosen at run time.
        body = f"CopyDevice.usesPadWords ? {swift_body(leaf.ipad, params, leaf.plural, nested=True)} : {swift_body(forms, params, leaf.plural, nested=True)}"
    if not params:
        lines.append(f"{indent}static var {name}: String {{")
    else:
        signature = ", ".join(f"{swift_ident(p)}: {'Int' if p == 'count' else 'String'}" for p in params)
        lines.append(f"{indent}static func {name}({signature}) -> String {{")
    lines.append(f"{indent}    {body}")
    lines.append(f"{indent}}}")
    return lines


def swift_body(forms: dict, params: list[str], plural: bool, nested: bool = False) -> str:
    """A form's Swift expression; a plural's in parentheses when `nested` in another conditional."""
    if not plural:
        return swift_literal(forms["text"], params)
    body = f"count == 1 ? {swift_literal(forms['one'], params)} : {swift_literal(forms['other'], params)}"
    return f"({body})" if nested else body


def swift_group(group: Group, indent: str) -> list[str]:
    lines = []
    for index, child in enumerate(group.children):
        if index:
            if isinstance(child, Group) or isinstance(group.children[index - 1], Group):
                lines.append("")
        if isinstance(child, Group):
            lines.append(f"{indent}enum {type_name(child.name)} {{")
            lines.extend(swift_group(child, indent + "    "))
            lines.append(f"{indent}}}")
        else:
            lines.extend(swift_member(child, indent))
    return lines


def render_swift(root: Group) -> str:
    lines = [
        f"// {HEADER}.",
        "// Every word a reader sees or hears (docs/native-apps.md §7.1). Change the JSON, then run the",
        "// script; design/scripts/check_copy.py keeps literals out of the app.",
        "",
        "import Foundation",
        "",
        "// swiftlint:disable all",
        "",
        "enum Copy {",
    ]
    lines.extend(swift_group(root, "    "))
    lines += [
        "}",
        "",
        "/// Which device's words a platform variant with an iPad form uses (docs/native-apps.md §6.1).",
        "enum CopyDevice {",
        "    /// True on an iPad. The app sets it once at launch, before anything reads the copy.",
        "    nonisolated(unsafe) static var isPad = false",
        "    /// Replaces `isPad` inside `CopyDevice.$isPadOverride.withValue(…)`. It's per task, so tests",
        "    /// running side by side can each check one device's words.",
        "    @TaskLocal static var isPadOverride: Bool?",
        "",
        "    static var usesPadWords: Bool { isPadOverride ?? isPad }",
        "}",
        "",
        "/// A count as the reader's locale writes it (\"1,234\").",
        "private func copyNumber(_ value: Int) -> String {",
        "    value.formatted()",
        "}",
        "",
    ]
    return "\n".join(lines)


# ---- Kotlin -----------------------------------------------------------------------------------

def kotlin_ident(name: str) -> str:
    return f"`{name}`" if name in KOTLIN_KEYWORDS else name


def kotlin_literal(text: str) -> str:
    out = []
    pos = 0
    for match in PLACEHOLDER.finditer(text):
        out.append(_kotlin_escape(text[pos:match.start()]))
        name = match.group(1)
        out.append(f"${{copyNumber({kotlin_ident(name)})}}" if name == "count" else f"${{{kotlin_ident(name)}}}")
        pos = match.end()
    out.append(_kotlin_escape(text[pos:]))
    return '"' + "".join(out) + '"'


def _kotlin_escape(text: str) -> str:
    return (text.replace("\\", "\\\\").replace('"', '\\"').replace("$", "\\$").replace("\n", "\\n")
            .replace("\r", "\\r").replace("\t", "\\t"))


def kotlin_doc(leaf: Leaf, indent: str) -> list[str]:
    parts = []
    if leaf.plural or leaf.params("android"):
        parts.append(leaf.describe("android"))
    if leaf.kind == "variant":
        for other in ("ios", "desktop"):
            parts.append(f"{PLATFORM_NAMES[other]}: {leaf.describe(other)}")
        if leaf.tablet:
            parts.append(f"Tablet: {_describe_forms(leaf.tablet, leaf.plural)}")
    if leaf.note:
        parts.append(leaf.note)
    # Kotlin comments nest: keep "/*" and "*/" out of them.
    return [f"{indent}/** {part.replace('*/', '* /').replace('/*', '/ *')} */" for part in parts]


def kotlin_member(leaf: Leaf, indent: str) -> list[str]:
    params = leaf.params("android")
    forms = leaf.texts["android"]
    lines = kotlin_doc(leaf, indent)
    name = kotlin_ident(leaf.name)
    if not params and not leaf.tablet:
        lines.append(f"{indent}const val {name} = {kotlin_literal(forms['text'])}")
        return lines
    body = kotlin_body(forms, leaf.plural)
    if leaf.tablet:
        # A tablet's words where a phone's differ, chosen at run time (docs/native-apps.md §6.1).
        body = f"if (CopyDevice.usesTabletWords) {kotlin_body(leaf.tablet, leaf.plural, nested=True)} else {kotlin_body(forms, leaf.plural, nested=True)}"
    if not params:
        lines.append(f"{indent}val {name}: String get() =")
    else:
        signature = ", ".join(f"{kotlin_ident(p)}: {'Int' if p == 'count' else 'String'}" for p in params)
        lines.append(f"{indent}fun {name}({signature}): String =")
    lines.append(f"{indent}    {body}")
    return lines


def kotlin_body(forms: dict, plural: bool, nested: bool = False) -> str:
    """A form's Kotlin expression; a plural's in parentheses when `nested` in another conditional."""
    if not plural:
        return kotlin_literal(forms["text"])
    body = f"if (count == 1) {kotlin_literal(forms['one'])} else {kotlin_literal(forms['other'])}"
    return f"({body})" if nested else body


def kotlin_group(group: Group, indent: str) -> list[str]:
    lines = []
    for index, child in enumerate(group.children):
        if index:
            if isinstance(child, Group) or isinstance(group.children[index - 1], Group):
                lines.append("")
        if isinstance(child, Group):
            lines.append(f"{indent}object {type_name(child.name)} {{")
            lines.extend(kotlin_group(child, indent + "    "))
            lines.append(f"{indent}}}")
        else:
            lines.extend(kotlin_member(child, indent))
    return lines


def render_kotlin(root: Group) -> str:
    lines = [
        f"// {HEADER}.",
        "// Every word a reader sees or hears (docs/native-apps.md §7.1). Change the JSON, then run the",
        "// script; design/scripts/check_copy.py keeps literals out of the app.",
        "@file:Suppress(\"ConstPropertyName\", \"PropertyName\", \"FunctionName\", \"unused\", \"ktlint\")",
        "",
        "package com.librespydus.app.core",
        "",
        "import java.text.NumberFormat",
        "",
        "object Copy {",
    ]
    lines.extend(kotlin_group(root, "    "))
    lines += [
        "}",
        "",
        "/** Which device's words a platform variant with a tablet form uses (docs/native-apps.md §6.1). */",
        "object CopyDevice {",
        "    /** True on a tablet (`smallestScreenWidthDp >= 600`). The app sets it once at launch, before anything reads the copy. */",
        "    @Volatile var isTablet: Boolean = false",
        "    /** Replaces [isTablet] while set: tests pin one device's words ([withTabletWords]). */",
        "    @Volatile var isTabletOverride: Boolean? = null",
        "",
        "    val usesTabletWords: Boolean get() = isTabletOverride ?: isTablet",
        "",
        "    /** Runs [block] with a tablet's (or a phone's) words, then restores the override that was set before. */",
        "    fun <T> withTabletWords(tablet: Boolean, block: () -> T): T {",
        "        val previous = isTabletOverride",
        "        isTabletOverride = tablet",
        "        try {",
        "            return block()",
        "        } finally {",
        "            isTabletOverride = previous",
        "        }",
        "    }",
        "}",
        "",
        "/** A count as the reader's locale writes it (\"1,234\"). */",
        "private fun copyNumber(value: Int): String = NumberFormat.getIntegerInstance().format(value)",
        "",
    ]
    return "\n".join(lines)


# ---- Rust -------------------------------------------------------------------------------------

def rust_ident(name: str) -> str:
    if name in RUST_NOT_RAW:
        return name + "_"
    return f"r#{name}" if name in RUST_KEYWORDS else name


def _rust_escape(text: str) -> str:
    return (text.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
            .replace("\r", "\\r").replace("\t", "\\t"))


def rust_param(name: str, consts: set) -> str:
    """A parameter's identifier: one named like a constant of its module would be read as a
    pattern matching that constant, so it gets a trailing "_"."""
    return name + "_" if name in consts else rust_ident(name)


def rust_expr(text: str, number: str, consts: set = frozenset()) -> str:
    """A `String` expression for `text`: `"…".to_owned()`, or a `format!` with one positional
    argument per placeholder (counts through `number`, the path to `copy_number`)."""
    pieces = []
    args = []
    pos = 0
    for match in PLACEHOLDER.finditer(text):
        pieces.append(_rust_escape(text[pos:match.start()]).replace("{", "{{").replace("}", "}}"))
        pieces.append("{}")
        name = match.group(1)
        args.append(f"{number}({rust_param(name, consts)})" if name == "count" else rust_param(name, consts))
        pos = match.end()
    pieces.append(_rust_escape(text[pos:]).replace("{", "{{").replace("}", "}}"))
    if not args:
        return f'"{_rust_escape(text)}".to_owned()'
    return f'format!("{"".join(pieces)}", {", ".join(args)})'


def rust_doc(leaf: Leaf, indent: str) -> list[str]:
    lines = []
    if leaf.plural or leaf.params("desktop"):
        lines.append(f"{indent}/// {leaf.describe('desktop')}")
    if leaf.kind == "variant":
        for other in ("ios", "android"):
            lines.append(f"{indent}/// {PLATFORM_NAMES[other]}: {leaf.describe(other)}")
    if leaf.note:
        lines.append(f"{indent}/// {leaf.note}")
    return lines


def rust_member(leaf: Leaf, indent: str, depth: int, consts: set) -> list[str]:
    params = leaf.params("desktop")
    forms = leaf.forms["desktop"]
    lines = rust_doc(leaf, indent)
    name = rust_ident(leaf.name)
    if not params:
        lines.append(f'{indent}pub const {name}: &str = "{_rust_escape(forms["text"])}";')
        return lines
    number = "super::" * depth + "copy_number"
    signature = ", ".join(f"{rust_param(p, consts)}: {'i64' if p == 'count' else '&str'}" for p in params)
    lines.append(f"{indent}pub fn {name}({signature}) -> String {{")
    if leaf.plural:
        lines.append(f"{indent}    if {rust_param('count', consts)} == 1 {{")
        lines.append(f"{indent}        {rust_expr(forms['one'], number, consts)}")
        lines.append(f"{indent}    }} else {{")
        lines.append(f"{indent}        {rust_expr(forms['other'], number, consts)}")
        lines.append(f"{indent}    }}")
    else:
        lines.append(f"{indent}    {rust_expr(forms['text'], number, consts)}")
    lines.append(f"{indent}}}")
    return lines


def rust_group(group: Group, indent: str, depth: int) -> list[str]:
    lines = []
    consts = {child.name for child in group.children if isinstance(child, Leaf) and not child.params("desktop")}
    for index, child in enumerate(group.children):
        if index:
            if isinstance(child, Group) or isinstance(group.children[index - 1], Group):
                lines.append("")
        if isinstance(child, Group):
            lines.append(f"{indent}pub mod {type_name(child.name)} {{")
            lines.extend(rust_group(child, indent + "    ", depth + 1))
            lines.append(f"{indent}}}")
        else:
            lines.extend(rust_member(child, indent, depth, consts))
    return lines


def render_rust(root: Group) -> str:
    lines = [
        f"// {HEADER}.",
        "// Every word a reader sees or hears (docs/native-apps.md §7.1, docs/desktop-app.md §8). Change",
        "// the JSON, then run the script; design/scripts/check_copy.py keeps literals out of the app.",
        "",
        "#![allow(non_snake_case, non_upper_case_globals, dead_code, clippy::all)]",
        "",
        "/// `Copy::Common::retry`, `Copy::Search::results(count)`: one module per group, a `const` per",
        "/// plain string and a function per string with placeholders or plural forms.",
        "pub mod Copy {",
    ]
    lines.extend(rust_group(root, "    ", 1))
    lines += [
        "}",
        "",
        "/// A count with digit grouping (\"1,234\").",
        "fn copy_number(value: i64) -> String {",
        "    let digits = value.unsigned_abs().to_string();",
        "    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3 + 1);",
        "    if value < 0 {",
        "        grouped.push('-');",
        "    }",
        "    for (index, digit) in digits.chars().enumerate() {",
        "        if index > 0 && (digits.len() - index) % 3 == 0 {",
        "            grouped.push(',');",
        "        }",
        "        grouped.push(digit);",
        "    }",
        "    grouped",
        "}",
        "",
    ]
    return "\n".join(lines)


# ---- main -------------------------------------------------------------------------------------

def outputs(root: Group) -> dict[Path, str]:
    return {SWIFT_OUT: render_swift(root), KOTLIN_OUT: render_kotlin(root), RUST_OUT: render_rust(root)}


def stats(root: Group) -> str:
    leaves = list(root.leaves())
    plurals = sum(leaf.plural for leaf in leaves)
    variants = sum(leaf.kind == "variant" for leaf in leaves)
    ipad = sum(leaf.ipad is not None for leaf in leaves)
    tablet = sum(leaf.tablet is not None for leaf in leaves)
    functions = sum(bool(leaf.params("ios")) for leaf in leaves)
    return (f"{len(leaves)} strings ({plurals} plurals, {variants} platform variants ({ipad} with an iPad form, "
            f"{tablet} with a tablet form), {functions} with placeholders) in {sum(1 for _ in _groups(root))} groups")


def _groups(group: Group):
    for child in group.children:
        if isinstance(child, Group):
            yield child
            yield from _groups(child)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--check", action="store_true", help="exit 1 when a generated file is out of date")
    args = parser.parse_args(argv)
    try:
        root = load()
    except CopyError as error:
        print(f"gen_copy: {error}", file=sys.stderr)
        return 2
    stale = []
    for path, text in outputs(root).items():
        current = path.read_text(encoding="utf-8") if path.exists() else None
        if current == text:
            continue
        stale.append(path)
        if not args.check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
    relative = [str(path.relative_to(ROOT)) for path in stale]
    if args.check:
        if stale:
            print("gen_copy: out of date (run design/scripts/gen_copy.py): " + ", ".join(relative), file=sys.stderr)
            return 1
        print(f"gen_copy: up to date · {stats(root)}")
        return 0
    print(("gen_copy: wrote " + ", ".join(relative)) if stale else "gen_copy: already up to date")
    print(f"gen_copy: {stats(root)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
