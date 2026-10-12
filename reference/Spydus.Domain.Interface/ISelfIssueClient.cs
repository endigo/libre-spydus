using System.Threading.Tasks;
using Refit;
using Spydus.Domain.DTO;

namespace Spydus.Domain.Interface;

[Headers(new string[] { "Authorisation: Bearer" })]
public interface ISelfIssueClient
{
	[Post("/api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/issue?alert_acknowledged={alertAcknowledged}")]
	[Headers(new string[] { "Content-Type: application/json; charset=UTF-8" })]
	Task<IApiResponse<SelfIssueResponseDto>> BarcodeSelfIssue([AliasAs("patronIrn")] string? patronIrn, [AliasAs("itemId")] string? itemId, [AliasAs("alertAcknowledged")] bool alertAcknowledged);

	[Post("/api/mobile/1.0/patrons/{patronIrn}/items/rfid/{itemRfid}/issue?alert_acknowledged={alertAcknowledged}")]
	[Headers(new string[] { "Content-Type: application/json; charset=UTF-8" })]
	Task<IApiResponse<SelfIssueResponseDto>> RfidSelfIssue([AliasAs("patronIrn")] string? patronIrn, [AliasAs("itemRfid")] string? itemId, [AliasAs("alertAcknowledged")] bool alertAcknowledged);

	[Post("/api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/return?alert_acknowledged={alertAcknowledged}")]
	[Headers(new string[] { "Content-Type: application/json; charset=UTF-8" })]
	Task<IApiResponse<SelfIssueResponseDto>> SelfReturn([AliasAs("patronIrn")] string? patronIrn, [AliasAs("itemId")] string? itemId, [AliasAs("alertAcknowledged")] bool alertAcknowledged);
}
