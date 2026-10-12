# Investigation record

Inspected 11 October 2026, Australia/Perth. Target package: `com.civica.civicaspydus`, version `2.0.6`, version code `784`, minimum Android SDK `29`, target SDK `36`. Orca `1.4.224` controlled the running `Pixel_10_Pro` (`emulator-5554`), an Android 17 production emulator image. The library selected in the signed-in app was City of Canning.

## Evidence and coverage

| Question | Result | Boundary |
| --- | --- | --- |
| Native routes, verbs, parameters and DTOs | 23 interface operations, 29 DTO types, 155 properties recovered | Static client declarations; does not enumerate every server route |
| Original app HTTP | 11 requests, 8 distinct operations, all HTTP 200 | Two bounded native captures, startup/account validation and keyword search |
| Identity/session bridge | APK OAuth settings and compiled flow; existing session validation observed | No fresh login, exchange, refresh, expiry or logout performed |
| Web catalogue | Public search, details and availability reads succeeded; scripts inspected | Other referenced AJAX/workflow paths remain static |
| Signed-in account | Account, current loans, pickup and pending reservations read successfully | Reproduced HTTP reads using captured session; not original WebView request attribution |
| Renewal/reservation actions | Exact selected/all renewal and cancellation links, edit and reserve references recovered | No actions submitted; resulting status/errors unverified |
| Self issue/return and wallet | Native declarations recovered | Canning config disables self issue/return; wallet requests not exercised |
| Flutter data mapping | JSON types plus account HTML field selectors documented | No Flutter application or HTML parser implemented yet |

The native capture includes session validation, server version, notifications, branches, configuration, containers, announcements, and keyword search. Canning returned empty containers/announcements in this session. Empty lists do not establish all possible item schemas; compiled DTOs provide separate static evidence.

The web inventory contains 36 route/action contracts. These include parameterized route families and conditional script references, rather than 36 successful HTTP operations. It distinguishes public reads, reconstructed authenticated reads, and unexecuted mutations.

One additional direct `HttpClient.GetAsync` call exists in Spydus.dll method `0x06000b60`. Its decoded diagnostics identify `DownloadCalendarEvent`; the URL is supplied dynamically, rather than a recovered fixed API route. Calendar downloads were not observed in this Canning session. Library logos, OPAC assets, remote localization assets, and externally supplied container URLs also require following server configuration rather than inventing fixed endpoints.

## Artifact identity

| Artifact | SHA-256 |
| --- | --- |
| Installed base APK | `b3e695955d02c2a252eaf600d44c6366fa6b563a76209d8261764d845a104f73` |
| Installed arm64 split APK | `61c1ca93b83cf99efeb7ebd6efba66a1ebd390046c81c5212e0de9c3119c0b56` |
| Extracted Spydus.dll | `e0e729df2c9cdd73d106a754905df152d98b020301e414c9756f31ecf1e7c3c7` |
| Extracted Spydus.Domain.dll | `b61f5c0faaace83d1c0097f0b1b027a949550eec6215333f263e87dc86b0411a` |

The arm64 APK holds `libassembly-store.so`, a .NET Android v3 assembly store. The extraction script recovered 289 managed assemblies and verified their PE signatures. Main string decoding recovered 8,686 uses of the observed helper with zero method decode failures; it only recognizes that helper/call pattern and is not a proof that every runtime string or dynamic URL was recovered.

## REA records

Complete records remain local, under `.private/`. Evidence IDs are content-bound analysis references, not credentials.

| Record | Evidence ID |
| --- | --- |
| Base APK inventory | `ev_9471796488aaa9d5e1628e1fed1cc687d9d645821b45dc0de1913be7f5b70515` |
| Domain managed artifact | `ev_6f08659f790a3e2fcd55bc8dff68378f4fa3faa8f05b506646f51e2bf6574c80` |
| Domain managed members | `ev_b0d73dc574706e8706b31d3e453a8a5974c72a4967476a534d8518419befe938` |
| Original app startup HTTP | `ev_5858e5f6d7e7fbac645d70945dda71cce88e89c025df33220ba25805188b632a` |
| Original app discovery HTTP | `ev_c7d43b85269c9600900a1c29359caba672b13f53a5f2b4851d73b215e68b2e96` |
| Final deidentified native HAR | `ev_ff39efc50c7c85c0485e0aff4703352bd3cddefff718880074cbaacf1e8cd023` |
| Public OPAC JavaScript | `ev_f97fd26a718f07d2f8f56c251b2d1857434ee5afe5b0878e7e072008930a24cc` |
| Reconstructed authenticated web reads | `ev_8a286dabbb192399dbd2b525716d630b0d32d1cc927bc8e365e4fffd444fe7c1` |

The authenticated web journal is curl-produced, not browser-produced. Request Cookie values were omitted by its producer; account response bodies/headers remain private. A derived `reads-normalized.har` percent-encodes URI-invalid characters in one producer URL and retains its pre-normalization value as provenance. REA admitted that normalized journal; this does not turn curl reads into original WebView observations.

Final managed-artifact and HAR checks ran with REA 6.4.0 and a temporary Node 24.21.0 runtime. Initial extraction used REA 6.3.0 through the host CLI. No global Node or agent configuration was changed.

## Capture and cleanup

Orca supplied device discovery, attachment, app navigation, accessibility trees, Android shell commands and package identity. Plain adb transferred the installed APKs and temporary certificate. REA inspected archive inventory, managed PE metadata/CIL, public JavaScript and retained HAR records. No JADX/native decompiler was required for the recovered .NET managed contracts.

For native HTTP capture, mitmproxy 12.2.3 ran locally, limited to Canning's host/resolved addresses. Emulator-level HTTP proxy routing exposed the original native requests. Captured session validation supplied the session used for subsequent read-only OPAC inspection. The original production WebView's certificate/debugging boundary prevented an equivalent full WebView body capture; its requests are not claimed as observed.

Cleanup restored the normal Orca emulator launch arguments, removed all temporary global proxy keys, removed the temporary user CA and copied certificate, and stopped the local proxy. The app's signed-in account was preserved. No screen lock was added and no library account mutations were submitted. The emulator remains booted.

Raw APKs/assemblies, account HTML, sessions, cookies, screenshots, captures and tool dependencies stay in ignored `.private/`; directory/file permissions restrict them to the local user. The shared fixture removes Authorization/Cookie headers, session values and account values, and redacts notification text/identifiers. Nothing was published or sent to another person.

## Reproduction

The scripts use local artifacts and do not execute the installed app unless explicitly operating the proxy addon. Contract extraction requires `dnfile`; assembly extraction requires `lz4`; string decoding requires `dnfile` and `dncil`.

```sh
.private/tools/venv/bin/python scripts/extract_assemblies.py \
  .private/apk/extracted/lib/arm64-v8a/libassembly-store.so .private/apk/assemblies
.private/tools/venv/bin/python scripts/extract_api_contract.py
python3 scripts/generate_postman.py
.private/tools/venv/bin/python scripts/decode_app_strings.py \
  .private/apk/assemblies/Spydus.dll .private/captures/app-string-evidence.json
python3 scripts/sanitize_capture.py \
  .private/captures/emulator.har .private/captures/discover.har \
  --output docs/api/fixtures/native-traffic.har
npx -y --package node@24 --package rea-agents@latest rea \
  inspect-web-network-capture "$PWD/docs/api/fixtures/native-traffic.har" har --json
```

The contract and collection generators were regenerated deterministically. Operation tokens, SHA-256 and MVID matched REA member/artifact evidence; body examples are typed JSON and all Postman placeholders are defined. A privacy scan checked reusable files against captured credentials, borrower IDs, account values, CSRF and authenticated result-set IDs. REA admitted both final HAR datasets.

Authoritative tool/format references used: [REA managed analysis](https://github.com/morluto/rea/blob/main/docs/managed-code-analysis.md), [REA historical network captures](https://github.com/morluto/rea/blob/main/docs/web-network-captures.md), [.NET Android assembly stores](https://github.com/dotnet/android/blob/main/Documentation/project-docs/AssemblyStores.md), and [Chromium WebView debugging boundary](https://github.com/chromium/chromium/blob/main/android_webview/docs/net-debugging.md). Orca's command reference was read from `orca skills get orca-cli` and `orca skills get orca-emulator-android`.
