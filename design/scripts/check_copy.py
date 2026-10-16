#!/usr/bin/env python3
"""Find user-facing string literals in the apps' code (docs/native-apps.md §7.1).

    design/scripts/check_copy.py            report every literal, exit 1 if there is any
    design/scripts/check_copy.py --summary  also count them per file
    design/scripts/check_copy.py PATH…      check only these files

Every word a reader sees or hears comes from design/copy.json through the generated Copy
(ios/LibreSpydus/Core/Copy.swift, android/…/core/Copy.kt, desktop/src/copy.rs). This scans
ios/LibreSpydus/**/*.swift, android/app/src/main/java/**/*.kt and desktop/src/**/*.rs and prints
`file:line: "literal"` for:

- a literal with letters in a UI position: SwiftUI Text / Label / Button / navigationTitle / alert /
  confirmationDialog / accessibilityLabel / Hint / Value / prompt / placeholder, notification
  content title / body; Compose Text, text =, contentDescription, label / placeholder,
  AlertDialog title / text, Snackbar, NotificationCompat setContentTitle / Text, channel names,
  onClickLabel, custom actions; GPUI / gpui-kit `.child(` / `.children(`, `.label(`,
  `.placeholder(`, `.tooltip(`, `.title(`, `.description(`, `Notification::*(`, `aria_label(` /
  `accessibility_label(`, menu items (and this app's own components: SectionHeader, Chip, Banner,
  EmptyStateView, LoadingView, the pill buttons, …);
- anywhere else in view or model code, a literal that reads like prose: words with a space, a
  capitalised word ("Retry"), or typographic punctuation (’ “ ” … ·).

Not reported: the generated Copy files; debug-only files (component galleries, debug hooks and
actions, demo backend and sample data, the timing log); Swift `#if DEBUG` blocks, Kotlin
`if (BuildConfig.DEBUG …) { … }` blocks and Rust items or blocks under `#[cfg(debug_assertions)]`
or `#[cfg(test)]` (a file with `#![cfg(debug_assertions)]`, and `if cfg!(debug_assertions) { … }`);
comments; log / print / exception / precondition messages (Rust: `log::…!`, `println!`,
`eprintln!`, `panic!`, `expect(`, `#[error(…)]`, `actions!`, `KeyBinding::new`, element `.id(`);
keys, identifiers, symbol and icon names, URLs, date patterns and other technical strings; and any
line marked `// copy-exempt: <reason>` (or the code line right after a comment line holding only
that marker). The reason is required.

The Rust lexer knows raw strings (`r#"…"#`), byte and C strings, char literals and lifetimes, and
reads `format!` placeholders (`{}`, `{name}`, `{0:>4}`) as interpolations.

Python 3.9+ standard library only (no `match`); no network.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IOS_SOURCES = ROOT / "ios" / "LibreSpydus"
ANDROID_SOURCES = ROOT / "android" / "app" / "src" / "main" / "java"
DESKTOP_SOURCES = ROOT / "desktop" / "src"

GENERATED = {
    ROOT / "ios" / "LibreSpydus" / "Core" / "Copy.swift",
    ANDROID_SOURCES / "com" / "librespydus" / "app" / "core" / "Copy.kt",
    DESKTOP_SOURCES / "copy.rs",
}

# Debug-only files: never shown to readers in a release build.
DEBUG_ONLY = [
    re.compile(r"(^|/)ComponentsGallery\.swift$"),
    re.compile(r"(^|/)ComponentGallery\.kt$"),
    re.compile(r"DebugHooks[^/]*\.(swift|kt)$"),
    re.compile(r"DebugActions[^/]*\.(swift|kt)$"),
    re.compile(r"(^|/)Demo(Backend|Data|Catalogue|Picks)\.(swift|kt)$"),
    re.compile(r"(^|/)TimingLog\.(swift|kt)$"),
    re.compile(r"(^|/)LaunchOptions\.(swift|kt)$"),
    # desktop: the timing log, launch flags, demo and fake backends, components gallery
    re.compile(r"^desktop/src/(timing|launch)\.rs$"),
    re.compile(r"^desktop/src/backend/(demo|demo_catalogue|fake)\.rs$"),
    re.compile(r"^desktop/src/ui/gallery\.rs$"),
]

EXEMPT = re.compile(r"//\s*copy-exempt:\s*\S")
EXEMPT_NO_REASON = re.compile(r"//\s*copy-exempt\b(?!:\s*\S)")

# The code right before a literal (same line, or the previous line when the literal opens its line)
# that puts it in a UI position.
UI_CALLS = (
    # SwiftUI
    r"Text|Label|Button|Toggle|Picker|DatePicker|TextField|SecureField|DisclosureGroup|Section|Link|"
    r"ShareLink|Menu|ContentUnavailableView|navigationTitle|alert|confirmationDialog|"
    r"accessibilityLabel|accessibilityHint|accessibilityValue|accessibilityInputLabels|"
    r"accessibilityAction|accessibilityCustomContent|Announcement|help|badge|searchable|"
    # Compose / Android
    r"CustomAccessibilityAction|setContentTitle|setContentText|setSubText|setTicker|setBigContentTitle|"
    r"setSummaryText|bigText|NotificationChannel|NotificationChannelGroup|showSnackbar|Snackbar|"
    r"makeText|announceForAccessibility|AlertDialog|DropdownMenuItem|Tab|"
    # this app's components (both platforms)
    r"SectionHeader|ProfileSectionHeader|AccountSectionHeader|Chip|Banner|AccountBanner|EmptyStateView|"
    r"ErrorStateView|LoadingView|SheetLoadingView|SheetLoading|PrimaryPillButton|SecondaryPillButton|"
    r"CircleIconButton|InfoRow|infoRow|retryLine|RetryLine|Footnote|ProfileRowButton|AccountRowButton|"
    r"AlertSwitchRow|SectionMessageCard|HomeSectionMessageCard|PlaceholderScreen|labelled|placeholder|"
    r"sectionTitle|SegmentedToggle|Notice|PicksNotice|UserFacingError"
)
UI_ARGS = (
    r"title|subtitle|message|text|label|labelText|placeholder|prompt|caption|hint|body|actionTitle|"
    r"actionLabel|accessibilityLabel|accessibilityHint|contentDescription|stateDescription|"
    r"onClickLabel|onLongClickLabel|description|footer|header|noResultsTitle|noResultsMessage|"
    r"refreshMessage|badge|menuTitle|named"
)
UI_BEFORE = re.compile(
    rf"(?:\b(?:{UI_CALLS})\s*(?:\(|\{{)\s*(?:verbatim:\s*)?$)"   # Text("…", Text(verbatim: "…", Button {
    rf"|(?:\b(?:{UI_ARGS})\s*[:=]\s*$)"                         # title: "…", contentDescription = "…"
    rf"|(?:\.(?:title|subtitle|body)\s*=\s*$)"                  # content.title = "…"
)

# Code right before a literal that makes it a key, a pattern or a message for developers.
NOT_UI_BEFORE = re.compile(
    r"(?:\b(?:Image|UIImage|NSImage|Color|UIColor)\(\s*(?:named:\s*)?$)"
    r"|(?:\b(?:systemImage|systemName|forResource|withExtension|forKey|key|identifier|"
    r"accessibilityIdentifier|testTag|tag|hasPrefix|hasSuffix|startsWith|endsWith|contains|"
    r"removePrefix|removeSuffix|range\(of|ofPattern|dateFormat|withTemplate|Regex|NSRegularExpression|"
    r"queryItem|getStringExtra|getBooleanExtra|putExtra|getString|putString|getStringSet|"
    r"putStringSet|getInstance|forName|Locale|TimeZone|identifier|path|appending\(path|"
    r"PREFS_NAME|threadIdentifier|categoryIdentifier|serviceName|service|account|Keychain\w*|"
    r"fields|DEFAULT|TAG|icon|ImageVector\.Builder|vectorResource|painterResource)\s*[\(\[:=]?\s*$)"
    r"|(?:\b(?:print|debugPrint|NSLog|os_log|fatalError|precondition|preconditionFailure|assert|"
    r"assertionFailure|require|requireNotNull|check|checkNotNull|error|TODO|Exception|Error|"
    r"IllegalStateException|IllegalArgumentException|UnsupportedOperationException|"
    r"NotLoggedIn|Upstream|Parse|Network|SuppressLint|Suppress|available|JvmName|"
    r"OptIn|Deprecated|SerialName|CodingKeys)\s*\(\s*(?:message:\s*)?$)"
    r"|(?:\b(?:Log|AlertLog|Logger|logger|log)\s*\.\s*\w+\s*\([^\"]*$)"
    r"|(?:\b(?:require|check|requireNotNull|checkNotNull|precondition|assert)\s*\(.*\)\s*\{\s*$)"
    r"|(?:\bsetOf\(|\bhashSetOf\(|\bSet<String>|\bmapOf\(.*\bto\s*$)"
    r"|(?:[=!]==?\s*$)|(?:\bcase\s+$)|(?:\bin\s+$)|(?:\bis\s+$)|(?:\[\s*$(?<=\w\[))"
)
LOG_LINE = re.compile(r"\b(?:print|debugPrint|NSLog|os_log|AlertLog\.print|Log\.[vdiwe]|Timber\.\w)\s*\(")

# ---- Rust (desktop/src) ----
# Code right before a literal that puts it in a GPUI / gpui-kit UI position.
RUST_UI_BEFORE = re.compile(
    r"(?:\.(?:child|children|label|placeholder|tooltip|title|subtitle|description|message|caption|hint|"
    r"text|ok_text|cancel_text|aria_label|accessibility_label|accessibility_hint|menu|menu_with_icon|"
    r"footer|header|default_value|prefix_label|suffix_label)\s*\(\s*$)"
    r"|(?:\b(?:Notification::\w+|Label::new|Tag::\w+|Badge::new|Link::new|MenuItem::\w+|Menu::new|"
    r"PopupMenuItem::new|SharedString::from|Tooltip::new|"
    # this app's components (src/ui)
    r"SectionHeader::new|Chip::new|Chip::\w+|Banner::new|EmptyStateView::new|ErrorStateView::new|"
    r"LoadingView::new|PrimaryPillButton::new|SecondaryPillButton::new|CircleIconButton::new|"
    r"GenreTile::new|ShelfCover::new|BookRowCard::new|ConfirmDialog::new|PlaceholderScreen::new|"
    r"toast::\w+|UserFacingError::new|aria_label|push_notification)\s*\(\s*$)"
    r"|(?:\b(?:name|title|label|message|text|caption|hint|placeholder|description)\s*:\s*$)"
)
# Code right before a literal that makes it a key, an id, a pattern or a developer message.
RUST_NOT_UI_BEFORE = re.compile(
    r"(?:\.(?:id|key_context|debug_selector|path|join|starts_with|ends_with|contains|strip_prefix|"
    r"strip_suffix|split|find|replace|trim_start_matches|trim_end_matches|get|header|env|var|"
    r"with_extension|extension)\s*\(\s*$)"
    r"|(?:\b(?:include_str|include_bytes|env|concat|matches|cfg|Regex::new|Path::new|PathBuf::from|"
    r"Entry::new|try_parse_color|ElementId::\w+|Keystroke::parse)!?\s*\(\s*$)"
    r"|(?:[=!]=\s*$)"
)
# Lines (statements) whose literals are never shown to a reader.
RUST_NOT_UI_LINE = re.compile(
    r"\b(?:log::\w+!|(?:e?print(?:ln)?|panic|unreachable|todo|unimplemented|assert\w*|debug_assert\w*|"
    r"write(?:ln)?|actions|anyhow|bail|ensure)!|expect\s*\(|expect_err\s*\(|KeyBinding::new\s*\("
    r")|#\[error\("
)

TYPOGRAPHIC = re.compile(r"[’‘“”…·—–]")
WORD = re.compile(r"[A-Za-z]{2,}")
DATE_PATTERN = re.compile(r"^[\sdDEMyuHhmsaLGQwWkKzZXx.,:/'\-]+$")
FORMAT_SPEC = re.compile(r"%(?:\d+\$)?[-+ 0#]*\d*(?:\.\d+)?[dfsx@]")
SVG_PATH = re.compile(r"^[MmLlHhVvCcSsQqTtAaZz]\s*-?\d[\d\s,.\-MmLlHhVvCcSsQqTtAaZz]*$")
IDENTIFIER = re.compile(r"^[A-Za-z0-9_$.]*[a-z0-9_$][A-Za-z0-9_$.]*[A-Z][A-Za-z0-9_$.]*$")  # camelCase keys
LOWER_TOKEN = re.compile(r"^[a-z0-9_.\-:/#@+=&?%*\[\]<>(){}|!~^$\\]+$")              # keys, symbols, paths
UPPER_TOKEN = re.compile(r"^[A-Z0-9_.\-:/#@+&=]+$")                                  # "AU", "UTC", "SHA-256"


@dataclass
class Literal:
    line: int
    source: str   # the literal as written, quotes included
    text: str     # its text with every interpolation replaced by {}


# ---- lexer ------------------------------------------------------------------------------------

class Lexer:
    """Splits Swift / Kotlin source into code and string literals, skipping comments.

    Handles nested block comments, escapes, Swift `\\(…)` and Kotlin `${…}` interpolation (with
    literals nested inside), Kotlin `$name`, multi-line `\"\"\"` strings, Swift raw `#"…"#`
    strings and Kotlin character literals.
    """

    def __init__(self, source: str, kotlin: bool, rust: bool = False):
        self.src = source
        self.kotlin = kotlin
        self.rust = rust
        self.literals: list[Literal] = []
        # The code with comments and literals blanked out (same length and line breaks), so a
        # line's code can be read without its strings.
        self.code = list(source)

    def run(self) -> "Lexer":
        self._code(0, len(self.src), stop=None)
        return self

    def _line(self, pos: int) -> int:
        return self.src.count("\n", 0, pos) + 1

    def _blank(self, start: int, end: int) -> None:
        for i in range(start, end):
            if self.code[i] != "\n":
                self.code[i] = " "

    def _code(self, pos: int, end: int, stop: str | None) -> int:
        """Scan code from pos; returns the index after the closing `stop` char, if any."""
        src = self.src
        depth = 0
        while pos < end:
            ch = src[pos]
            if src.startswith("//", pos):
                close = src.find("\n", pos)
                close = end if close == -1 else close
                self._blank(pos, close)
                pos = close
                continue
            if src.startswith("/*", pos):
                pos = self._block_comment(pos)
                continue
            if self.rust:
                if ch == '"' or (ch in "rbc" and _rust_string_start(src, pos)):
                    pos = self._rust_string(pos)
                    continue
                if ch == "'":
                    pos = self._rust_quote(pos)
                    continue
                if stop is not None:
                    if ch in "({[":
                        depth += 1
                    elif ch in ")}]":
                        if depth == 0 and ch == stop:
                            return pos + 1
                        depth -= 1
                pos += 1
                continue
            if ch == '"' or (ch == "#" and not self.kotlin and re.match(r'#+"', src[pos:])):
                pos = self._string(pos)
                continue
            if self.kotlin and ch == "'":
                close = pos + 1
                while close < end and src[close] != "'":
                    close += 2 if src[close] == "\\" else 1
                pos = close + 1
                continue
            if stop is not None:
                if ch in "({[":
                    depth += 1
                elif ch in ")}]":
                    if depth == 0 and ch == stop:
                        return pos + 1
                    depth -= 1
            pos += 1
        return pos

    def _rust_quote(self, pos: int) -> int:
        """A char literal (`'a'`, `'\\n'`, `'\\u{2026}'`, `'…'`) is skipped; a lifetime or label (`'a`,
        `'static`) is code."""
        src = self.src
        if src.startswith("\\", pos + 1):
            close = src.find("'", pos + 3)
            return len(src) if close == -1 else close + 1
        if pos + 2 < len(src) and src[pos + 2] == "'" and src[pos + 1] != "\n":
            return pos + 3
        return pos + 1

    def _rust_string(self, pos: int) -> int:
        """`"…"`, `b"…"`, `c"…"`, `r"…"`, `r#"…"#`, `br##"…"##`: escapes in the plain ones, none in
        the raw ones; may span lines. `{…}` format placeholders read as interpolations, `{{` as `{`."""
        src = self.src
        start = pos
        while src[pos] in "bc":
            pos += 1
        raw = src[pos] == "r"
        if raw:
            pos += 1
        hashes = 0
        while src[pos] == "#":
            hashes += 1
            pos += 1
        pos += 1  # the opening quote
        closing = '"' + "#" * hashes
        text: list[str] = []
        while pos < len(src):
            if src.startswith(closing, pos):
                pos += len(closing)
                break
            ch = src[pos]
            if ch == "\\" and not raw and pos + 1 < len(src):
                nxt = src[pos + 1]
                if nxt == "\n":  # line continuation: the newline and the next line's indent go
                    pos += 2
                    while pos < len(src) and src[pos] in " \t\n":
                        pos += 1
                    continue
                if nxt == "u":
                    close = src.find("}", pos)
                    try:
                        text.append(chr(int(src[pos + 3:close], 16)))
                    except ValueError:
                        pass
                    pos = close + 1
                    continue
                text.append({"n": "\n", "t": "\t", "r": "\r", "0": "", '"': '"', "'": "'", "\\": "\\"}.get(nxt, nxt))
                pos += 2
                continue
            text.append(ch)
            pos += 1
        self._blank(start, pos)
        body = "".join(text)
        # Format placeholders ({}, {name}, {0:>4}, {:?}) are interpolations; {{ and }} are braces.
        body = re.sub(r"\{\{|\}\}|\{[A-Za-z0-9_.]*(?::[^{}]*)?\}",
                      lambda m: {"{{": "{", "}}": "}"}.get(m.group(0), "{}"), body)
        self.literals.append(Literal(self._line(start), src[start:pos], body))
        return pos

    def _block_comment(self, pos: int) -> int:
        src = self.src
        start = pos
        depth = 0
        while pos < len(src):
            if src.startswith("/*", pos):
                depth += 1
                pos += 2
            elif src.startswith("*/", pos):
                depth -= 1
                pos += 2
                if depth == 0:
                    break
            else:
                pos += 1
        self._blank(start, pos)
        return pos

    def _string(self, pos: int) -> int:
        src = self.src
        start = pos
        hashes = 0
        while src[pos] == "#":
            hashes += 1
            pos += 1
        multiline = src.startswith('"""', pos)
        quote = '"""' if multiline else '"'
        pos += len(quote)
        closing = quote + "#" * hashes
        text: list[str] = []
        raw_kotlin = self.kotlin and multiline
        while pos < len(src):
            if src.startswith(closing, pos):
                pos += len(closing)
                break
            ch = src[pos]
            if not multiline and ch == "\n":
                break  # unterminated: give up on this line
            if ch == "\\" and not raw_kotlin and src.startswith("\\" + "#" * hashes, pos):
                nxt = pos + 1 + hashes
                if not self.kotlin and nxt < len(src) and src[nxt] == "(":
                    pos = self._code(nxt + 1, len(src), stop=")")
                    text.append("{}")
                    continue
                if nxt < len(src):
                    text.append({"n": "\n", "t": "\t", '"': '"', "\\": "\\", "$": "$"}.get(src[nxt], src[nxt]))
                    pos = nxt + 1
                    continue
            if self.kotlin and ch == "$":
                if src.startswith("${", pos):
                    pos = self._code(pos + 2, len(src), stop="}")
                    text.append("{}")
                    continue
                name = re.match(r"\$[A-Za-z_][A-Za-z0-9_]*", src[pos:])
                if name:
                    pos += name.end()
                    text.append("{}")
                    continue
            text.append(ch)
            pos += 1
        self._blank(start, pos)
        self.literals.append(Literal(self._line(start), src[start:pos], "".join(text)))
        return pos


# ---- rules ------------------------------------------------------------------------------------

def _rust_string_start(src: str, pos: int) -> bool:
    """`r"`, `r#"`, `b"`, `br"`, `br#"`, `c"`, `cr"` at `pos`, and not the end of an identifier."""
    if pos > 0 and (src[pos - 1].isalnum() or src[pos - 1] == "_"):
        return False
    return bool(re.match(r'(?:b|c)?r#*"|(?:b|c)"', src[pos:pos + 40]))


def reads_like_prose(text: str) -> bool:
    interpolated = "{}" in text
    bare = text.replace("{}", "").strip()
    probe = text.replace("{}", "x").strip()  # "x profile" is prose, "due:x:x" a key
    if interpolated and "·" in bare:
        return True  # "{title} · {branch}": a line built in code
    if not WORD.search(bare):
        return False
    if is_technical(probe):
        return False
    if TYPOGRAPHIC.search(bare) or interpolated:
        return True  # "{name} profile", "by {date}", "Version {v} ({b})"
    if re.search(r"[A-Za-z][\"')]?\s+[\"'(]?[A-Za-z{]|[A-Za-z][.!?:]\s|^[A-Za-z]+[.!?]$|^\d+\s+[A-Za-z]{2,}", bare):
        return True  # words; "1 result"
    return bool(re.fullmatch(r"[A-Z][a-z]+(?:[-'][A-Za-z]+)*", bare))  # "Retry", "Sign-in"


def is_technical(bare: str, ui: bool = False) -> bool:
    if "://" in bare or bare.startswith(("http", "/", "./", "../", "file:", "mailto:")):
        return True
    if DATE_PATTERN.match(bare) and re.search(r"\b(?:d{1,2}|M{3,4}|E{3,4}|y{2,4}|u{4}|HH|mm)\b", bare):
        return True
    if FORMAT_SPEC.search(bare) and not re.search(r"[a-z]{3,}\s[a-z]{3,}", bare):
        return True
    if SVG_PATH.match(bare) or LOWER_TOKEN.match(bare) or UPPER_TOKEN.match(bare):
        return True
    if IDENTIFIER.match(bare) and not (ui and bare[:1].isupper()):  # in a Text( it's a name: "LibreSpydus"
        return True
    if re.search(r"[=;]\s*\w+[=;]|\bMax-Age=|\bPath=/", bare):  # cookies, query strings
        return True
    if re.search(r"\\[sdwbS]|\(\?|\[\^|\.\*|\\\.", bare):       # regular expressions
        return True
    if re.fullmatch(r"[\w.]+\.(?:json|png|jpg|pdf|plist|txt|html|css|js|kt|swift)", bare):
        return True
    return False


def in_ui_position(before: str) -> bool:
    return bool(UI_BEFORE.search(before))


def not_ui(before: str) -> bool:
    return bool(NOT_UI_BEFORE.search(before))


# ---- files ------------------------------------------------------------------------------------

def debug_regions_swift(lines: list[str]) -> set[int]:
    """Line numbers inside `#if DEBUG` branches (not their #else)."""
    skipped: set[int] = set()
    stack: list[bool] = []  # per #if: is the current branch debug-only?
    for number, raw in enumerate(lines, 1):
        line = raw.strip()
        if line.startswith("#if"):
            condition = line[3:]
            stack.append(bool(re.search(r"(?<![!\w])DEBUG\b", condition)) and "||" not in condition)
            skipped.add(number)
            continue
        if line.startswith("#elseif") or line.startswith("#else"):
            if stack:
                stack[-1] = False
            skipped.add(number)
            continue
        if line.startswith("#endif"):
            if stack:
                stack.pop()
            skipped.add(number)
            continue
        if any(stack):
            skipped.add(number)
    return skipped


def debug_regions_kotlin(code_lines: list[str]) -> set[int]:
    """Lines of `if (BuildConfig.DEBUG …) { … }` blocks, or the one line of `if (BuildConfig.DEBUG) x`."""
    skipped: set[int] = set()
    index = 0
    while index < len(code_lines):
        line = code_lines[index]
        match = re.search(r"\bif\s*\(\s*BuildConfig\.DEBUG\b(?![^)]*\|\|)[^)]*\)", line)
        if not match:
            index += 1
            continue
        rest = line[match.end():]
        if rest.strip() and not rest.strip().startswith("{"):
            skipped.add(index + 1)
            index += 1
            continue
        depth = 0
        opened = False
        while index < len(code_lines):
            skipped.add(index + 1)
            segment = code_lines[index][match.end():] if not opened else code_lines[index]
            for ch in segment:
                if ch == "{":
                    depth += 1
                    opened = True
                elif ch == "}":
                    depth -= 1
            index += 1
            if opened and depth <= 0:
                break
    return skipped


def _rust_cfg_is_debug(condition: str) -> bool:
    """`debug_assertions`, `test`, `all(…, debug_assertions, …)`, `any(test, debug_assertions)`."""
    condition = condition.replace(" ", "")
    if "not(" in condition:
        return False
    if condition.startswith("all("):
        return "debug_assertions" in condition or re.search(r"\btest\b", condition) is not None
    names = re.findall(r"[A-Za-z_][A-Za-z0-9_]*(?:=\"[^\"]*\")?", condition)
    names = [name for name in names if name != "any"]
    return bool(names) and all(name in ("debug_assertions", "test") for name in names)


def _rust_item_end(code: str, pos: int) -> int:
    """Where the item or statement starting at `pos` ends: after its `;` or `,`, or after the
    `{…}` block it opens (brackets and parens nest; a closure inside them doesn't end it)."""
    depth = 0
    while pos < len(code):
        ch = code[pos]
        if ch in "([":
            depth += 1
        elif ch in ")]":
            if depth == 0:
                return pos
            depth -= 1
        elif depth == 0 and ch in ";,":
            return pos + 1
        elif depth == 0 and ch == "}":
            return pos
        elif depth == 0 and ch == "{":
            braces = 0
            while pos < len(code):
                if code[pos] == "{":
                    braces += 1
                elif code[pos] == "}":
                    braces -= 1
                    if braces == 0:
                        return pos + 1
                pos += 1
            return pos
        pos += 1
    return pos


def debug_regions_rust(code: str) -> set[int]:
    """Lines under `#[cfg(debug_assertions)]` / `#[cfg(test)]` (the item or statement after it),
    `if cfg!(debug_assertions) { … }` blocks, or the whole file with `#![cfg(debug_assertions)]`."""
    skipped: set[int] = set()

    def mark(start: int, end: int) -> None:
        first = code.count("\n", 0, start) + 1
        last = code.count("\n", 0, end) + 1
        skipped.update(range(first, last + 1))

    for match in re.finditer(r"#!\[cfg\((.*?)\)\]", code):
        if _rust_cfg_is_debug(match.group(1)):
            mark(0, len(code))
            return skipped
    for match in re.finditer(r"#\[cfg\((.*?)\)\]", code):
        if not _rust_cfg_is_debug(match.group(1)):
            continue
        pos = match.end()
        # Skip the item's other attributes.
        while True:
            rest = re.match(r"\s*#\[", code[pos:])
            if not rest:
                break
            pos = _rust_item_end(code, pos + rest.end())  # to the attribute's closing ]
            pos += 1
        mark(match.start(), _rust_item_end(code, pos))
    for match in re.finditer(r"\bif\s+cfg!\(\s*debug_assertions\s*\)\s*\{", code):
        mark(match.start(), _rust_item_end(code, match.end() - 1))
    return skipped


@dataclass
class Finding:
    path: Path
    line: int
    literal: str
    reason: str


def check_file(path: Path) -> list[Finding]:
    if path.suffix == ".rs":
        return check_rust_file(path)
    kotlin = path.suffix == ".kt"
    source = path.read_text(encoding="utf-8")
    lexer = Lexer(source, kotlin).run()
    raw_lines = source.split("\n")
    code_lines = "".join(lexer.code).split("\n")
    skipped = debug_regions_kotlin(code_lines) if kotlin else debug_regions_swift(raw_lines)
    findings = []
    for literal in lexer.literals:
        number = literal.line
        if number in skipped or exempt(raw_lines, number):
            continue
        line_code = code_lines[number - 1]
        if LOG_LINE.search(line_code):
            continue
        column = _column(raw_lines[number - 1], literal.source)
        before = line_code[:column] if column >= 0 else line_code
        if not before.strip() and number > 1:
            before = code_lines[number - 2].rstrip() + " " + before
        before = before.rstrip()
        bare = literal.text.replace("{}", "").strip()
        if not re.search(r"[A-Za-z·]", bare):
            continue
        if not_ui(before):
            continue
        if in_ui_position(before) and re.search(r"[A-Za-z]", bare) and not is_technical(bare, ui=True):
            findings.append(Finding(path, number, literal.source, "ui"))
        elif reads_like_prose(literal.text):
            findings.append(Finding(path, number, literal.source, "prose"))
    return findings


def check_rust_file(path: Path) -> list[Finding]:
    source = path.read_text(encoding="utf-8")
    lexer = Lexer(source, kotlin=False, rust=True).run()
    raw_lines = source.split("\n")
    code = "".join(lexer.code)
    code_lines = code.split("\n")
    skipped = debug_regions_rust(code)
    findings = []
    for literal in lexer.literals:
        number = literal.line
        if number in skipped or exempt(raw_lines, number):
            continue
        line_code = code_lines[number - 1]
        column = _column(raw_lines[number - 1], literal.source)
        before = line_code[:column] if column >= 0 else line_code
        if not before.strip() and number > 1:
            before = code_lines[number - 2].rstrip() + " " + before
        if RUST_NOT_UI_LINE.search(before) or RUST_NOT_UI_LINE.search(line_code):
            continue
        before = before.rstrip()
        bare = literal.text.replace("{}", "").strip()
        if not re.search(r"[A-Za-z·]", bare) or literal.source.startswith(("b", "br")):
            continue  # byte strings are data, never shown
        if RUST_NOT_UI_BEFORE.search(before):
            continue
        if RUST_UI_BEFORE.search(before) and not is_technical(bare, ui=True):
            findings.append(Finding(path, number, literal.source, "ui"))
        elif reads_like_prose(literal.text):
            findings.append(Finding(path, number, literal.source, "prose"))
    return findings


def _column(line: str, literal_source: str) -> int:
    first = literal_source.split("\n", 1)[0]
    return line.find(first)


def exempt(lines: list[str], number: int) -> bool:
    if EXEMPT.search(lines[number - 1]):
        return True
    previous = lines[number - 2].strip() if number > 1 else ""
    return previous.startswith("//") and bool(EXEMPT.match(previous))


def sources() -> list[Path]:
    files = (sorted(IOS_SOURCES.rglob("*.swift")) + sorted(ANDROID_SOURCES.rglob("*.kt"))
             + sorted(DESKTOP_SOURCES.rglob("*.rs")))
    return [path for path in files if path not in GENERATED and not is_debug_only(path)]


def is_debug_only(path: Path) -> bool:
    relative = path.relative_to(ROOT).as_posix()
    return any(pattern.search(relative) for pattern in DEBUG_ONLY)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("paths", nargs="*", type=Path, help="files to check (default: every app)")
    parser.add_argument("--summary", action="store_true", help="also print a count per file")
    args = parser.parse_args(argv)
    paths = [path.resolve() for path in args.paths] if args.paths else sources()
    findings: list[Finding] = []
    unmarked: list[str] = []
    for path in paths:
        if path in GENERATED:
            continue
        findings.extend(check_file(path))
        for number, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
            if EXEMPT_NO_REASON.search(line):
                unmarked.append(f"{_display(path)}:{number}: copy-exempt needs a reason (// copy-exempt: <reason>)")
    for finding in findings:
        literal = finding.literal.replace("\n", "\\n")
        print(f"{_display(finding.path)}:{finding.line}: {literal}")
    for message in unmarked:
        print(message)
    if args.summary and findings:
        print()
        for path, count in Counter(_display(f.path) for f in findings).most_common():
            print(f"{count:4d}  {path}")
    files = len({f.path for f in findings})
    if findings or unmarked:
        print(f"\ncheck_copy: {len(findings)} user-facing literal(s) in {files} file(s)"
              + (f", {len(unmarked)} exemption(s) without a reason" if unmarked else "")
              + " — move them to design/copy.json (§7.1) or mark the line // copy-exempt: <reason>")
        return 1
    print(f"check_copy: no user-facing literals in {len(paths)} file(s)")
    return 0


def _display(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


if __name__ == "__main__":
    sys.exit(main())
