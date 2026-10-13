#!/usr/bin/env python3
"""Generate the apps' Copy files from design/copy.json (docs/native-apps.md §7.1).

    design/scripts/gen_copy.py           write both files
    design/scripts/gen_copy.py --check   exit 1 when either file is out of date (writes nothing)

Writes ios/LibreSpydus/Core/Copy.swift (`enum Copy { enum Picks { static let title = … } }`) and
android/app/src/main/java/com/librespydus/app/core/Copy.kt (`object Copy { object Picks { const
val title = … } }`), with the same names on both platforms:

- each JSON object becomes a nested enum / object (`yourBooks` → `YourBooks`), each leaf a member
  with the leaf's camelCase key;
- a string without placeholders is a constant; one with {named} placeholders is a function whose
  labelled parameters follow the order of first appearance (`count: Int`, every other one
  `String`); {count} is shown with the locale's digit grouping ("1,234 results");
- {"one": …, "other": …} is a function of `count: Int` (first) plus the other placeholders,
  choosing "one" when count == 1;
- {"ios": …, "android": …} resolves to that platform's text (both must use the same placeholders);
- keys starting with "_" are notes: "_notes" entries become doc comments, the rest are skipped.

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


class CopyError(Exception):
    pass


# ---- model ------------------------------------------------------------------------------------

class Leaf:
    """One string: plain, plural or per platform, resolved for one platform."""

    def __init__(self, path: list[str], value, note: str | None):
        self.path = path
        self.name = path[-1]
        self.note = note
        self.raw = value
        where = ".".join(path)
        if isinstance(value, str):
            self.kind = "text"
            self.texts = {"ios": {"text": value}, "android": {"text": value}}
        elif isinstance(value, dict) and set(value) == {"one", "other"}:
            self.kind = "plural"
            forms = {"one": value["one"], "other": value["other"]}
            if not all(isinstance(text, str) for text in forms.values()):
                raise CopyError(f"{where}: plural forms must be strings")
            if "count" not in placeholders(forms["other"]):
                raise CopyError(f"{where}: a plural's \"other\" form must use {{count}}")
            self.texts = {"ios": forms, "android": forms}
        elif isinstance(value, dict) and set(value) == {"ios", "android"}:
            self.kind = "platform"
            if not all(isinstance(text, str) for text in value.values()):
                raise CopyError(f"{where}: platform variants must be strings")
            if placeholders(value["ios"]) != placeholders(value["android"]):
                raise CopyError(f"{where}: the ios and android texts must use the same placeholders")
            self.texts = {"ios": {"text": value["ios"]}, "android": {"text": value["android"]}}
        else:
            raise CopyError(f"{where}: a leaf is a string, {{\"one\", \"other\"}} or {{\"ios\", \"android\"}}")
        for forms in self.texts.values():
            for text in forms.values():
                if not text.strip():
                    raise CopyError(f"{where}: empty text")
                if any(brace in PLACEHOLDER.sub("", text) for brace in "{}"):
                    raise CopyError(f"{where}: stray brace in {text!r}")

    def params(self, platform: str) -> list[str]:
        """Placeholders in order of first appearance; a plural's count first."""
        forms = self.texts[platform]
        names: list[str] = ["count"] if self.kind == "plural" else []
        for key in ("other", "one") if self.kind == "plural" else ("text",):
            for name in placeholders(forms[key]):
                if name not in names:
                    names.append(name)
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
    return set(value) in ({"one", "other"}, {"ios", "android"})


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
    forms = leaf.texts[platform]
    if leaf.kind == "plural":
        lines.append(f'{indent}/// One: "{forms["one"]}" · other: "{forms["other"]}"')
    elif leaf.params(platform):
        lines.append(f'{indent}/// "{forms["text"]}"')
    if leaf.kind == "platform":
        lines.append(f'{indent}/// Android: "{leaf.texts["android"]["text"]}"')
    if leaf.note:
        lines.append(f"{indent}/// {leaf.note}")
    return lines


def swift_member(leaf: Leaf, indent: str) -> list[str]:
    params = leaf.params("ios")
    forms = leaf.texts["ios"]
    lines = swift_doc(leaf, indent)
    name = swift_ident(leaf.name)
    if not params:
        lines.append(f"{indent}static let {name} = {swift_literal(forms['text'], params)}")
        return lines
    signature = ", ".join(f"{swift_ident(p)}: {'Int' if p == 'count' else 'String'}" for p in params)
    if leaf.kind == "plural":
        body = (f"count == 1 ? {swift_literal(forms['one'], params)} : "
                f"{swift_literal(forms['other'], params)}")
    else:
        body = swift_literal(forms["text"], params)
    lines.append(f"{indent}static func {name}({signature}) -> String {{")
    lines.append(f"{indent}    {body}")
    lines.append(f"{indent}}}")
    return lines


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
    forms = leaf.texts["android"]
    if leaf.kind == "plural":
        parts.append(f'One: "{forms["one"]}" · other: "{forms["other"]}"')
    elif leaf.params("android"):
        parts.append(f'"{forms["text"]}"')
    if leaf.kind == "platform":
        parts.append(f'iOS: "{leaf.texts["ios"]["text"]}"')
    if leaf.note:
        parts.append(leaf.note)
    # Kotlin comments nest: keep "/*" and "*/" out of them.
    return [f"{indent}/** {part.replace('*/', '* /').replace('/*', '/ *')} */" for part in parts]


def kotlin_member(leaf: Leaf, indent: str) -> list[str]:
    params = leaf.params("android")
    forms = leaf.texts["android"]
    lines = kotlin_doc(leaf, indent)
    name = kotlin_ident(leaf.name)
    if not params:
        lines.append(f"{indent}const val {name} = {kotlin_literal(forms['text'])}")
        return lines
    signature = ", ".join(f"{kotlin_ident(p)}: {'Int' if p == 'count' else 'String'}" for p in params)
    if leaf.kind == "plural":
        body = f"if (count == 1) {kotlin_literal(forms['one'])} else {kotlin_literal(forms['other'])}"
    else:
        body = kotlin_literal(forms["text"])
    lines.append(f"{indent}fun {name}({signature}): String =")
    lines.append(f"{indent}    {body}")
    return lines


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
        "/** A count as the reader's locale writes it (\"1,234\"). */",
        "private fun copyNumber(value: Int): String = NumberFormat.getIntegerInstance().format(value)",
        "",
    ]
    return "\n".join(lines)


# ---- main -------------------------------------------------------------------------------------

def outputs(root: Group) -> dict[Path, str]:
    return {SWIFT_OUT: render_swift(root), KOTLIN_OUT: render_kotlin(root)}


def stats(root: Group) -> str:
    leaves = list(root.leaves())
    plurals = sum(leaf.kind == "plural" for leaf in leaves)
    variants = sum(leaf.kind == "platform" for leaf in leaves)
    functions = sum(bool(leaf.params("ios")) for leaf in leaves)
    return (f"{len(leaves)} strings ({plurals} plurals, {variants} platform variants, "
            f"{functions} with placeholders) in {sum(1 for _ in _groups(root))} groups")


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
