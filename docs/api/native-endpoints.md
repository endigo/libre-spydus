# Native API contracts

Recovered 23 declared operations from the installed app's managed assembly. 8 operations have separate observations from original app HTTP traffic; the remaining 15 are static contracts only.

Assembly SHA-256: `b61f5c0faaace83d1c0097f0b1b027a949550eec6215333f263e87dc86b0411a`

Module MVID: `f5b2c27d-5e38-4019-b651-70f670a74ed8`

REA evidence:

- `inspect_managed_artifact`: `ev_6f08659f790a3e2fcd55bc8dff68378f4fa3faa8f05b506646f51e2bf6574c80`
- `inspect_managed_members`: `ev_b0d73dc574706e8706b31d3e453a8a5974c72a4967476a534d8518419befe938`

Original-app HTTP capture evidence (raw files stay private):

- `emulator.har`: 8 matched request(s); SHA-256 `16a270987bad76b5457450766a40d85fff01c29f3386219bce0f1ad8c083db96`.
- `discover.har`: 3 matched request(s); SHA-256 `a53107b08c68a1a97d480a722d8e3b215f8175419560e5376d493b11e887e1d5`.

Reproduce with `.private/tools/venv/bin/python scripts/extract_api_contract.py`. Machine-readable definitions, explicit attributes, parameter types and evidence tokens are in [native-endpoints.json](native-endpoints.json). Parameterized requests are in [spydus.postman_collection.json](spydus.postman_collection.json).

## Operations

| Interface / method | Verb | Exact route template | Evidence |
| --- | --- | --- | --- |
| `IAuthClient.GetAccessToken` | GET | `/openid/refreshtoken` | static |
| `IAuthClient.GetRefreshToken` | GET | `/openid/token` | static |
| `IBookClient.GetSearchBook` | GET | `/api/mobile/1.0/discover/search?{searchMode}={searchData}` | static + observed |
| `ILibrariesClient.GetLibraries` | GET | `/mobile/libraries?groupCode={groupCode}` | static |
| `ILibrariesClient.GetLanguages` | GET | `/mobile/languages?groupCode={groupCode}` | static |
| `ILogoClient.GetLogo` | GET | `/api/mobile/1.0/general/logo` | static |
| `ILogOutClient.LogOut` | POST | `/openid/patronlogout` | static |
| `INotificationClient.GetPatronNotifications` | GET | `/api/mobile/1.0/patrons/{brwdIrn}/notifications` | static + observed |
| `INotificationClient.UpdateNotificationStatus` | PATCH | `/api/mobile/1.0/patrons/{brwdIrn}/notifications/{id}/{status}` | static |
| `INotificationClient.DeleteDeviceToken` | DELETE | `/api/mobile/1.0/{patronIrn}/notifications/device/{deviceToken}` | static |
| `INotificationClient.PostDeviceToken` | POST | `/api/mobile/1.0/patrons/{patronIrn}/notifications/device` | static |
| `INotificationClient.PutDeviceToken` | PUT | `/api/mobile/1.0/patrons/{patronIrn}/notifications/device` | static |
| `ISelfIssueClient.BarcodeSelfIssue` | POST | `/api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/issue?alert_acknowledged={alertAcknowledged}` | static |
| `ISelfIssueClient.RfidSelfIssue` | POST | `/api/mobile/1.0/patrons/{patronIrn}/items/rfid/{itemRfid}/issue?alert_acknowledged={alertAcknowledged}` | static |
| `ISelfIssueClient.SelfReturn` | POST | `/api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/return?alert_acknowledged={alertAcknowledged}` | static |
| `IStyleClient.GetServerVersion` | GET | `/api/open/1.0/general/version` | static + observed |
| `IStyleClient.GetStyleConfiguration` | GET | `/api/mobile/1.0/general/config?wifiName={wifiName}` | static + observed |
| `IStyleClient.GetContainers` | GET | `/api/mobile/1.0/containers?wifiName={wifiName}` | static + observed |
| `IStyleClient.GetAnnouncements` | GET | `/api/mobile/1.0/announcements` | static + observed |
| `IStyleClient.GetAppleWalletCard` | GET | `/api/mobile/1.0/patrons/{patronId}/wallet/ios` | static |
| `IStyleClient.GetGoogleWalletCard` | GET | `/api/mobile/1.0/patrons/{patronId}/wallet/google?language={isoCode}` | static |
| `IStyleClient.GetBranches` | GET | `/api/mobile/1.0/mobile/branches?setId={setId}&lastIrn={lastIrn}&count={count}` | static + observed |
| `IValidateSessionClient.ValidateSessionId` | POST | `/api/private/1.0/patrons/validatesessionid` | static + observed |

The declarations preserve several unusual details: token exchange and refresh use GET; the device DELETE route omits `/patrons`; the branches route repeats `/mobile`; and the self issue interface declares `Authorisation: Bearer` with that exact spelling and no token parameter. Additional HTTP handler behavior or server support for those declarations requires separate evidence.

## Request and response bindings

### GetAccessToken

`GET /openid/refreshtoken`

Declared return: `Task<IApiResponse<AccessTokenResponse>>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `refreshToken` | `String` | query-default `refreshToken` (inferred; not captured) |
| `clientId` | `String` | query-default `clientId` (inferred; not captured) |

### GetRefreshToken

`GET /openid/token`

Declared return: `Task<IApiResponse<RefreshTokenResponse>>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `authorizationCode` | `String` | query-default `code` (inferred; not captured) |
| `grantType` | `String` | query-default `grant_type` (inferred; not captured) |
| `clientId` | `String` | query-default `client_id` (inferred; not captured) |

### GetSearchBook

`GET /api/mobile/1.0/discover/search?{searchMode}={searchData}`

Declared return: `Task<String>`.

Observed in original app traffic: 1 request(s), HTTP 200, JSON string response.

query `keywords=Harry Potter`; header `spydus-lang: `; JSON-string handoff to `/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ`; no result records in the native response.

| Parameter | Type | Binding |
| --- | --- | --- |
| `searchMode` | `String` | query-template `{searchMode}` |
| `searchData` | `String` | query-template `{searchData}` |
| `languageIdentifier` | `String` | header `spydus-lang` |

### GetLibraries

`GET /mobile/libraries?groupCode={groupCode}`

Declared return: `Task<IApiResponse<List<LibraryDto>>>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `groupCode` | `String` | query-template `{groupCode}` |

### GetLanguages

`GET /mobile/languages?groupCode={groupCode}`

Declared return: `Task<IApiResponse<List<AppLanguageDto>>>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `groupCode` | `String` | query-template `{groupCode}` |

### GetLogo

`GET /api/mobile/1.0/general/logo`

Declared return: `Task<IApiResponse<String>>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `cancellationToken` | `CancellationToken` | cancellation |

### LogOut

`POST /openid/patronlogout`

Declared return: `Task<IApiResponse>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `refreshToken` | `String` | query-default `refreshToken` (inferred; not captured) |
| `clientId` | `String` | query-default `clientId` (inferred; not captured) |

### GetPatronNotifications

`GET /api/mobile/1.0/patrons/{brwdIrn}/notifications`

Declared return: `Task<IApiResponse<List<NotificationResponseDto>>>`.

Observed in original app traffic: 2 request(s), HTTP 200, array response.

header `Authorization: Bearer <redacted>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `bearerToken` | `String` | authorization `Authorization: Bearer <token>` |
| `borrowerConsortiumRecordIdentifier` | `Int64` | path-template `{brwdIrn}` |

### UpdateNotificationStatus

`PATCH /api/mobile/1.0/patrons/{brwdIrn}/notifications/{id}/{status}`

Declared return: `Task<IApiResponse<Boolean>>`.

Declared headers: `Content-Type: application/json; charset=UTF-8`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `bearerToken` | `String` | authorization `Authorization: Bearer <token>` |
| `borrowerConsortiumRecordIdentifier` | `Int64` | path-template `{brwdIrn}` |
| `id` | `Guid` | path-template `{id}` |
| `status` | `String` | path-template `{status}` |

### DeleteDeviceToken

`DELETE /api/mobile/1.0/{patronIrn}/notifications/device/{deviceToken}`

Declared return: `Task<IApiResponse>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `bearerToken` | `String` | authorization `Authorization: Bearer <token>` |
| `borrowerLocalAccountIdentifier` | `String` | path-template `{patronIrn}` |
| `deviceToken` | `String` | path-template `{deviceToken}` |

### PostDeviceToken

`POST /api/mobile/1.0/patrons/{patronIrn}/notifications/device`

Declared return: `Task<IApiResponse>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `bearerToken` | `String` | authorization `Authorization: Bearer <token>` |
| `borrowerLocalAccountIdentifier` | `String` | path-template `{patronIrn}` |
| `deviceDto` | `RegisterDeviceTokenRequest` | body |

### PutDeviceToken

`PUT /api/mobile/1.0/patrons/{patronIrn}/notifications/device`

Declared return: `Task<IApiResponse>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `bearerToken` | `String` | authorization `Authorization: Bearer <token>` |
| `borrowerLocalAccountIdentifier` | `String` | path-template `{patronIrn}` |
| `request` | `UpdateDeviceTokenRequest` | body |

### BarcodeSelfIssue

`POST /api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/issue?alert_acknowledged={alertAcknowledged}`

Declared return: `Task<IApiResponse<SelfIssueResponseDto>>`.

Declared headers: `Authorisation: Bearer`, `Content-Type: application/json; charset=UTF-8`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `patronIrn` | `String` | path-template `{patronIrn}` |
| `itemId` | `String` | path-template `{itemId}` |
| `alertAcknowledged` | `Boolean` | query-template `{alertAcknowledged}` |

### RfidSelfIssue

`POST /api/mobile/1.0/patrons/{patronIrn}/items/rfid/{itemRfid}/issue?alert_acknowledged={alertAcknowledged}`

Declared return: `Task<IApiResponse<SelfIssueResponseDto>>`.

Declared headers: `Authorisation: Bearer`, `Content-Type: application/json; charset=UTF-8`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `patronIrn` | `String` | path-template `{patronIrn}` |
| `itemId` | `String` | path-template `{itemRfid}` |
| `alertAcknowledged` | `Boolean` | query-template `{alertAcknowledged}` |

### SelfReturn

`POST /api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/return?alert_acknowledged={alertAcknowledged}`

Declared return: `Task<IApiResponse<SelfIssueResponseDto>>`.

Declared headers: `Authorisation: Bearer`, `Content-Type: application/json; charset=UTF-8`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `patronIrn` | `String` | path-template `{patronIrn}` |
| `itemId` | `String` | path-template `{itemId}` |
| `alertAcknowledged` | `Boolean` | query-template `{alertAcknowledged}` |

### GetServerVersion

`GET /api/open/1.0/general/version`

Declared return: `Task<IApiResponse<ServerInfo>>`.

Observed in original app traffic: 1 request(s), HTTP 200, object response.

JSON response keys `client`, `isCloudHosted`, `license`, `patch`, `release`, `server`.

### GetStyleConfiguration

`GET /api/mobile/1.0/general/config?wifiName={wifiName}`

Declared return: `Task<IApiResponse<String>>`.

Observed in original app traffic: 1 request(s), HTTP 200, object response.

query `wifiName=`; JSON response keys `barcodeType`, `branchUrl`, `discovery`, `distanceUnit`, `enableJoinOnline`, `joinOnlineIcon`, `langs`, `libraryAppTimeZone`, `logoUrl`, `selfIssueEnabled`, `selfIssueWiFiOnly`, `selfReturnEnabled`, `selfReturnWiFiOnly`, `showNearbyBranches`, `style`, `wifi`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `wifiName` | `String` | query-template `{wifiName}` |

### GetContainers

`GET /api/mobile/1.0/containers?wifiName={wifiName}`

Declared return: `Task<IApiResponse<ContainerResponse>>`.

Observed in original app traffic: 1 request(s), HTTP 200, object response.

query `wifiName=`; header `spydus-lang: `; JSON response keys `containers`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `languageIdentifier` | `String` | header `spydus-lang` |
| `wifiName` | `String` | query-template `{wifiName}` |

### GetAnnouncements

`GET /api/mobile/1.0/announcements`

Declared return: `Task<IApiResponse<AnnouncementResponseDto>>`.

Observed in original app traffic: 1 request(s), HTTP 200, object response.

header `spydus-lang: `; JSON response keys `announcements`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `languageIdentifier` | `String` | header `spydus-lang` |

### GetAppleWalletCard

`GET /api/mobile/1.0/patrons/{patronId}/wallet/ios`

Declared return: `Task<HttpContent>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `bearerToken` | `String` | authorization `Authorization: Bearer <token>` |
| `patronId` | `String` | path-template `{patronId}` |

### GetGoogleWalletCard

`GET /api/mobile/1.0/patrons/{patronId}/wallet/google?language={isoCode}`

Declared return: `Task<IApiResponse<String>>`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `bearerToken` | `String` | authorization `Authorization: Bearer <token>` |
| `patronId` | `String` | path-template `{patronId}` |
| `isoCode` | `String` | query-template `{isoCode}` |

### GetBranches

`GET /api/mobile/1.0/mobile/branches?setId={setId}&lastIrn={lastIrn}&count={count}`

Declared return: `Task<IApiResponse<BranchResponseDto>>`.

Observed in original app traffic: 1 request(s), HTTP 200, object response.

query `setId=0`; query `lastIrn=0`; query `count=500`; header `spydus-lang: `; JSON response keys `branches`, `setInfo`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `setId` | `Int64` | query-template `{setId}` |
| `lastIrn` | `Int64` | query-template `{lastIrn}` |
| `count` | `Int32` | query-template `{count}` |
| `languageIdentifier` | `String` | header `spydus-lang` |

### ValidateSessionId

`POST /api/private/1.0/patrons/validatesessionid`

Declared return: `Task<IApiResponse<ValidateSessionResponse>>`.

Observed in original app traffic: 3 request(s), HTTP 200, object response.

header `Content-Type: application/json; charset=UTF-8`; JSON body keys `sessionId`; JSON response keys `barcode`, `brwIrn`, `brwSessionId`, `brwdIrn`, `institutionId`, `name`.

Declared headers: `Content-Type: application/json; charset=UTF-8`.

| Parameter | Type | Binding |
| --- | --- | --- |
| `request` | `ValidateSessionRequest` | body |

## DTO properties

Explicit JSON names are shown below. An unresolved name requires the application's runtime serializer policy. Ignored properties are application helpers, not declared serialized fields. Nullable value types are recorded as `Nullable<T>`; reference-type nullability is not asserted.

### AnnouncementResponseDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Announcements` | `List<AnnouncementDto>` | unresolved naming policy |

### AnnouncementDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Title` | `String` | unresolved naming policy |
| `StartDate` | `Nullable<DateTime>` | unresolved naming policy |
| `EndDate` | `Nullable<DateTime>` | unresolved naming policy |
| `Content` | `String` | unresolved naming policy |
| `HtmlSource` | `Microsoft.Maui.Controls.HtmlWebViewSource` | ignored (`JsonIgnore`) |

### AppLanguageDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `IsoCode` | `String` | `iso-code` |
| `LanguagePac` | `String` | `language-pac` |
| `Name` | `String` | unresolved naming policy |
| `LanguageName` | `String` | unresolved naming policy |

### BranchAddressDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `NumberAndStreet` | `String` | unresolved naming policy |
| `StreetLine2` | `String` | unresolved naming policy |
| `StreetLine3` | `String` | unresolved naming policy |
| `Suburb` | `String` | unresolved naming policy |
| `Postcode` | `String` | unresolved naming policy |
| `State` | `String` | unresolved naming policy |
| `Country` | `String` | unresolved naming policy |
| `FullAddress` | `String` | unresolved naming policy |

### BranchDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Irn` | `Int32` | unresolved naming policy |
| `Id` | `String` | unresolved naming policy |
| `Name` | `String` | unresolved naming policy |
| `Latitude` | `Nullable<Double>` | unresolved naming policy |
| `Longitude` | `Nullable<Double>` | unresolved naming policy |
| `Url` | `String` | unresolved naming policy |
| `OpacUrl` | `String` | unresolved naming policy |
| `ImageUrl` | `String` | unresolved naming policy |
| `Phone` | `String` | unresolved naming policy |
| `Email` | `String` | unresolved naming policy |
| `Address` | `BranchAddressDto` | unresolved naming policy |
| `OpeningHours` | `List<OpeningHourDto>` | unresolved naming policy |
| `Holidays` | `List<HolidayDto>` | unresolved naming policy |

### BranchResponseDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `SetInfo` | `SetInfo` | unresolved naming policy |
| `Branches` | `List<BranchDto>` | unresolved naming policy |

### HolidayDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `StartDate` | `String` | unresolved naming policy |
| `EndDate` | `String` | unresolved naming policy |
| `Name` | `String` | unresolved naming policy |
| `StartDateTime` | `String` | ignored (`JsonIgnore`) |
| `EndDateTime` | `String` | ignored (`JsonIgnore`) |
| `DisplayDateRange` | `String` | ignored (`JsonIgnore`) |

### OpeningHourDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Date` | `Object` | unresolved naming policy |
| `Dow` | `Int32` | unresolved naming policy |
| `DayName` | `String` | unresolved naming policy |
| `OpenTime` | `String` | unresolved naming policy |
| `CloseTime` | `String` | unresolved naming policy |
| `IsOpen` | `Boolean` | unresolved naming policy |
| `DisplayTime` | `String` | ignored (`JsonIgnore`) |
| `ShouldHighlightToday` | `Boolean` | ignored (`JsonIgnore`) |

### SetInfo

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `SetId` | `Int64` | unresolved naming policy |
| `LastIrn` | `Int64` | unresolved naming policy |
| `SetTotal` | `Int32` | unresolved naming policy |
| `SetCount` | `Int32` | unresolved naming policy |
| `HasMoreRecords` | `Boolean` | unresolved naming policy |

### ContainerResponse

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Containers` | `List<ContainerDto>` | unresolved naming policy |

### ContainerDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Title` | `String` | unresolved naming policy |
| `LoginRequired` | `Nullable<Int64>` | unresolved naming policy |
| `Icon` | `String` | unresolved naming policy |
| `Url` | `String` | unresolved naming policy |
| `Type` | `String` | unresolved naming policy |
| `IsExternalLink` | `Boolean` | ignored (`JsonIgnore`) |
| `OnlyDisplayWhenLoggedIn` | `Boolean` | ignored (`JsonIgnore`) |
| `OnlyDisplayWhenNotLoggedIn` | `Boolean` | ignored (`JsonIgnore`) |

### DiscoveryDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `SearchType` | `String` | unresolved naming policy |
| `DefaultSort` | `String` | unresolved naming policy |
| `AdvancedSearchUrl` | `String` | unresolved naming policy |

### LanguageDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Desc` | `String` | `desc` |
| `Id` | `String` | `id` |
| `ISO` | `String` | `iso` |
| `LanguageName` | `String` | unresolved naming policy |

### LibraryDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `ActivationCode` | `String` | unresolved naming policy |
| `Id` | `Guid` | unresolved naming policy |
| `Location` | `LocationDto` | unresolved naming policy |
| `Logo` | `String` | unresolved naming policy |
| `Name` | `String` | unresolved naming policy |
| `PinningHash` | `String[]` | unresolved naming policy |
| `Url` | `String` | unresolved naming policy |

### LocationDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Country` | `String` | unresolved naming policy |
| `State` | `String` | unresolved naming policy |
| `Latitude` | `Nullable<Double>` | unresolved naming policy |
| `Longitude` | `Nullable<Double>` | unresolved naming policy |

### Network

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Actions` | `List<String>` | unresolved naming policy |
| `Description` | `String` | unresolved naming policy |
| `Location` | `String` | unresolved naming policy |
| `Name` | `String` | unresolved naming policy |

### NotificationDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `BorrowerConsortiumRecordIdentifier` | `Int64` | `brwdIrn` |
| `BorrowerLocalAccountIdentifier` | `Int64` | `brwIrn` |
| `CreatedDate` | `DateTimeOffset` | `createdDate` |
| `Id` | `Guid` | `ntfId` |
| `InstCode` | `String` | `instCode` |
| `IsRead` | `Boolean` | `isRead` |
| `LibraryId` | `String` | `libraryId` |
| `Message` | `String` | `message` |
| `Subject` | `String` | `subject` |
| `Url` | `String` | `url` |

### NotificationResponseDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Notifications` | `List<NotificationDto>` | `notifications` |
| `TotalCount` | `Int64` | `totalCount` |

### RegisterDeviceTokenRequest

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `DeviceToken` | `String` | `deviceToken` |
| `Platform` | `String` | `platform` |
| `Tags` | `List<String>` | `tags` |

### SelfIssueAlertDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Code` | `String` | unresolved naming policy |
| `Critical` | `Boolean` | unresolved naming policy |
| `Irn` | `Int32` | unresolved naming policy |
| `Ref` | `String` | unresolved naming policy |
| `Text` | `String` | unresolved naming policy |

### SelfIssueResponseDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Alerts` | `List<SelfIssueAlertDto>` | unresolved naming policy |
| `BibAuthor` | `String` | unresolved naming policy |
| `BibIssueTitle` | `String` | unresolved naming policy |
| `BibTitle` | `String` | unresolved naming policy |
| `CoverImage` | `String` | unresolved naming policy |
| `DueDate` | `Nullable<DateTime>` | unresolved naming policy |
| `IsIssued` | `Nullable<Boolean>` | unresolved naming policy |
| `ItemCheckedIn` | `Nullable<Boolean>` | unresolved naming policy |
| `ItemId` | `String` | unresolved naming policy |
| `ItemIrn` | `Nullable<Int32>` | unresolved naming policy |
| `ItemRfid` | `String` | unresolved naming policy |
| `LoanIrn` | `Nullable<Int32>` | unresolved naming policy |
| `PatronIrn` | `Nullable<Int32>` | unresolved naming policy |
| `RentalAmount` | `Nullable<Double>` | unresolved naming policy |

### ServerInfo

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Client` | `String` | unresolved naming policy |
| `Release` | `String` | unresolved naming policy |
| `License` | `String` | unresolved naming policy |
| `Server` | `String` | unresolved naming policy |
| `Patch` | `String` | unresolved naming policy |

### StyleConfigDto

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `BarcodeType` | `String` | `barcodeType` |
| `BranchUrl` | `String` | unresolved naming policy |
| `ContentContainerUrl` | `String` | `contentContainerUrl` |
| `Langs` | `List<LanguageDto>` | `langs` |
| `LogoUrl` | `String` | `logoUrl` |
| `NotificationHubConnectionString` | `String` | `notificationHubConnectionString` |
| `NotificationHubName` | `String` | `notificationHubName` |
| `SelfIssueEnabled` | `Nullable<Boolean>` | `selfIssueEnabled` |
| `SelfReturnEnabled` | `Nullable<Boolean>` | `selfReturnEnabled` |
| `EnableJoinOnline` | `Nullable<Boolean>` | `enableJoinOnline` |
| `SpydusViewOverride` | `String` | `spydusViewOverride` |
| `Style` | `Dictionary<String, String>` | `style` |
| `Wifi` | `Wifi` | `wifi` |
| `Discovery` | `DiscoveryDto` | unresolved naming policy |
| `JoinOnlineIcon` | `String` | unresolved naming policy |
| `DistanceUnit` | `String` | unresolved naming policy |
| `LibraryAppTimeZone` | `String` | unresolved naming policy |
| `ShowNearbyBranches` | `Nullable<Boolean>` | unresolved naming policy |

### UpdateDeviceTokenRequest

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `NewDeviceToken` | `String` | `newDeviceToken` |
| `OldDeviceToken` | `String` | `oldDeviceToken` |
| `Platform` | `String` | `platform` |

### ValidateSessionRequest

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `BorrowerSessionId` | `String` | `sessionId` |

### ValidateSessionResponse

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `InstitutionId` | `String` | `institutionId` |
| `Name` | `String` | `name` |
| `Barcode` | `String` | `barcode` |
| `BorrowerLocalAccountIdentifier` | `String` | `brwIrn` |
| `BorrowerConsortiumRecordIdentifier` | `String` | `brwdIrn` |
| `BorrowerSessionId` | `String` | `brwSessionId` |

### Wifi

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `Networks` | `List<Network>` | unresolved naming policy |

### AccessTokenResponse

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `AccessToken` | `String` | `accessToken` |
| `RefreshToken` | `String` | `refreshToken` |

### RefreshTokenResponse

| CLR property | Type | JSON name / status |
| --- | --- | --- |
| `AccessToken` | `String` | `access_token` |
| `RefreshToken` | `String` | `refresh_token` |

## Limits

- This is not a complete inventory of webview, JavaScript, native platform, or dynamically constructed requests.
- Base URLs, interceptors, cookies, bearer values, serializer configuration, and actual server responses are not established by interface metadata.
- Parameters lacking explicit binding attributes use inferred Refit defaults; verify them against traffic before implementing.
- A null json_name means no explicit JsonPropertyNameAttribute; the runtime serializer naming policy still determines the wire name.
- Reference-type nullability and property defaults are not inferred; System.Nullable<T> is decoded from signatures.
- DTO schemas describe declared properties only, not validation, required fields, computed property behavior, or server guarantees.
