using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Refit;
using Spydus.Domain.DTO;

namespace Spydus.Domain.Interface;

public interface INotificationClient
{
	[Get("/api/mobile/1.0/patrons/{brwdIrn}/notifications")]
	Task<IApiResponse<List<NotificationResponseDto>>> GetPatronNotifications([Authorize("Bearer")] string bearerToken, [AliasAs("brwdIrn")] long borrowerConsortiumRecordIdentifier);

	[Patch("/api/mobile/1.0/patrons/{brwdIrn}/notifications/{id}/{status}")]
	[Headers(new string[] { "Content-Type: application/json; charset=UTF-8" })]
	Task<IApiResponse<bool>> UpdateNotificationStatus([Authorize("Bearer")] string bearerToken, [AliasAs("brwdIrn")] long borrowerConsortiumRecordIdentifier, [AliasAs("id")] Guid id, [AliasAs("status")] string status);

	[Delete("/api/mobile/1.0/{patronIrn}/notifications/device/{deviceToken}")]
	Task<IApiResponse> DeleteDeviceToken([Authorize("Bearer")] string bearerToken, [AliasAs("patronIrn")] string? borrowerLocalAccountIdentifier, [AliasAs("deviceToken")] string? deviceToken);

	[Post("/api/mobile/1.0/patrons/{patronIrn}/notifications/device")]
	Task<IApiResponse> PostDeviceToken([Authorize("Bearer")] string? bearerToken, [AliasAs("patronIrn")] string? borrowerLocalAccountIdentifier, [Body] RegisterDeviceTokenRequest deviceDto);

	[Put("/api/mobile/1.0/patrons/{patronIrn}/notifications/device")]
	Task<IApiResponse> PutDeviceToken([Authorize("Bearer")] string? bearerToken, [AliasAs("patronIrn")] string? borrowerLocalAccountIdentifier, [Body] UpdateDeviceTokenRequest request);
}
