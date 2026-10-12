import 'package:dio/dio.dart' hide Headers;
import 'package:retrofit/retrofit.dart';

import '../models/models.dart';

part 'patron_client.g.dart';

/// Host #3 — the selected library's Spydus server, authenticated endpoints.
/// Mirrors `INotificationClient`, `ISelfIssueClient`, and `IStyleClient` wallet.
///
/// The `Authorization: Bearer <accessToken>` header is injected centrally by
/// [AuthInterceptor] on the Dio instance passed in, matching the app's
/// `DynamicAuthenticatedApiClientFactory` + bearer DelegatingHandler.
///
/// Identifier note: notification paths use `{brwdIrn}` (consortium id); device
/// and self-issue paths use `{patronIrn}` (local id). Keep them distinct.
@RestApi()
abstract class PatronClient {
  factory PatronClient(Dio dio, {String? baseUrl}) = _PatronClient;

  // --- Notifications ---

  /// `INotificationClient.GetPatronNotifications`
  @GET('/api/mobile/1.0/patrons/{brwdIrn}/notifications')
  Future<NotificationResponseDto> getNotifications(@Path('brwdIrn') int brwdIrn);

  /// `INotificationClient.UpdateNotificationStatus`
  @PATCH('/api/mobile/1.0/patrons/{brwdIrn}/notifications/{id}/{status}')
  @Headers({'Content-Type': 'application/json; charset=UTF-8'})
  Future<bool> updateNotificationStatus(
    @Path('brwdIrn') int brwdIrn,
    @Path('id') String id,
    @Path('status') String status,
  );

  /// `INotificationClient.PostDeviceToken`
  @POST('/api/mobile/1.0/patrons/{patronIrn}/notifications/device')
  Future<void> registerDeviceToken(
    @Path('patronIrn') String patronIrn,
    @Body() RegisterDeviceTokenRequest request,
  );

  /// `INotificationClient.PutDeviceToken`
  @PUT('/api/mobile/1.0/patrons/{patronIrn}/notifications/device')
  Future<void> updateDeviceToken(
    @Path('patronIrn') String patronIrn,
    @Body() UpdateDeviceTokenRequest request,
  );

  /// `INotificationClient.DeleteDeviceToken` — note the route omits `/patrons`.
  @DELETE('/api/mobile/1.0/{patronIrn}/notifications/device/{deviceToken}')
  Future<void> deleteDeviceToken(
    @Path('patronIrn') String patronIrn,
    @Path('deviceToken') String deviceToken,
  );

  // --- Self service (issue / return) ---

  /// `ISelfIssueClient.BarcodeSelfIssue`
  @POST('/api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/issue')
  @Headers({'Content-Type': 'application/json; charset=UTF-8'})
  Future<SelfIssueResponseDto> barcodeSelfIssue(
    @Path('patronIrn') String patronIrn,
    @Path('itemId') String itemId,
    @Query('alert_acknowledged') bool alertAcknowledged,
  );

  /// `ISelfIssueClient.RfidSelfIssue`
  @POST('/api/mobile/1.0/patrons/{patronIrn}/items/rfid/{itemRfid}/issue')
  @Headers({'Content-Type': 'application/json; charset=UTF-8'})
  Future<SelfIssueResponseDto> rfidSelfIssue(
    @Path('patronIrn') String patronIrn,
    @Path('itemRfid') String itemRfid,
    @Query('alert_acknowledged') bool alertAcknowledged,
  );

  /// `ISelfIssueClient.SelfReturn`
  @POST('/api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/return')
  @Headers({'Content-Type': 'application/json; charset=UTF-8'})
  Future<SelfIssueResponseDto> selfReturn(
    @Path('patronIrn') String patronIrn,
    @Path('itemId') String itemId,
    @Query('alert_acknowledged') bool alertAcknowledged,
  );

  // --- Membership card (wallet) ---

  /// `IStyleClient.GetAppleWalletCard` — returns a `.pkpass` binary.
  @GET('/api/mobile/1.0/patrons/{patronId}/wallet/ios')
  @DioResponseType(ResponseType.bytes)
  Future<List<int>> getAppleWalletCard(@Path('patronId') String patronId);

  /// `IStyleClient.GetGoogleWalletCard` — returns a Google Wallet save URL string.
  @GET('/api/mobile/1.0/patrons/{patronId}/wallet/google')
  Future<String> getGoogleWalletCard(
    @Path('patronId') String patronId,
    @Query('language') String? isoCode,
  );
}
