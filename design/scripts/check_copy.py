#!/usr/bin/env python3
"""Find user-facing string literals in the apps' code (docs/native-apps.md §7.1).

    design/scripts/check_copy.py            report every literal, exit 1 if there is any
    design/scripts/check_copy.py --summary  also count them per file
    design/scripts/check_copy.py PATH…      check only these files

Every word a reader sees or hears comes from design/copy.json through the generated Copy
(ios/LibreSpydus/Core/Copy.swift, android/…/core/Copy.kt). This scans ios/LibreSpydus/**/*.swift and
android/app/src/main/java/**/*.kt and prints `file:line: "literal"` for:

- a literal with letters in a UI position: SwiftUI Text / Label / Button / navigationTitle / alert /
  confirmationDialog / accessibilityLabel / Hint / Value / prompt / placeholder, notification
  content title / body; Compose Text, text =, contentDescription, label / placeholder,
  AlertDialog title / text, Snackbar, NotificationCompat setContentTitle / Text, channel names,
  onClickLabel, custom actions (and this app's own components: SectionHeader, Chip, Banner,
  EmptyStateView, LoadingView, the pill buttons, …);
- anywhere else in view or model code, a literal that reads like prose: words with a space, a
  capitalised word ("Retry"), or typographic punctuation (’ “ ” … ·).

Not reported: the generated Copy files; debug-only files (component galleries, debug hooks and
actions, demo backend and sample data, the timing log); Swift `#if DEBUG` blocks and Kotlin
`if (BuildConfig.DEBUG …) { … }` blocks; comments; log / print / exception / precondition messages;
keys, identifiers, symbol and icon names, URLs, date patterns and other technical strings; and any
line marked `// copy-exempt: <reason>` (or the code line right after a comment line holding only
that marker). The reason is required.

Python 3 standard library only; no network.
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

GENERATED = {
    ROOT / "ios" / "LibreSpydus" / "Core" / "Copy.swift",
    ANDROID_SOURCES / "com" / "librespydus" / "app" / "core" / "Copy.kt",
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

    def __init__(self, source: str, kotlin: bool):
        self.src = source
        self.kotlin = kotlin
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


@dataclass
class Finding:
    path: Path
    line: int
    literal: str
    reason: str


def check_file(path: Path) -> list[Finding]:
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


def _column(line: str, literal_source: str) -> int:
    first = literal_source.split("\n", 1)[0]
    return line.find(first)


def exempt(lines: list[str], number: int) -> bool:
    if EXEMPT.search(lines[number - 1]):
        return True
    previous = lines[number - 2].strip() if number > 1 else ""
    return previous.startswith("//") and bool(EXEMPT.match(previous))


def sources() -> list[Path]:
    files = sorted(IOS_SOURCES.rglob("*.swift")) + sorted(ANDROID_SOURCES.rglob("*.kt"))
    return [path for path in files if path not in GENERATED and not is_debug_only(path)]


def is_debug_only(path: Path) -> bool:
    relative = path.relative_to(ROOT).as_posix()
    return any(pattern.search(relative) for pattern in DEBUG_ONLY)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("paths", nargs="*", type=Path, help="files to check (default: both apps)")
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
