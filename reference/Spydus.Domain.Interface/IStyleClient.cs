using System.Net.Http;
using System.Threading.Tasks;
using Refit;
using Spydus.Domain.DTO;

namespace Spydus.Domain.Interface;

public interface IStyleClient
{
	[Get("/api/open/1.0/general/version")]
	Task<IApiResponse<ServerInfo>> GetServerVersion();

	[Get("/api/mobile/1.0/general/config?wifiName={wifiName}")]
	Task<IApiResponse<string>> GetStyleConfiguration([AliasAs("wifiName")] string? wifiName);

	[Get("/api/mobile/1.0/containers?wifiName={wifiName}")]
	Task<IApiResponse<ContainerResponse>> GetContainers([Header("spydus-lang")] string? languageIdentifier, [AliasAs("wifiName")] string? wifiName);

	[Get("/api/mobile/1.0/announcements")]
	Task<IApiResponse<AnnouncementResponseDto>> GetAnnouncements([Header("spydus-lang")] string? languageIdentifier);

	[Get("/api/mobile/1.0/patrons/{patronId}/wallet/ios")]
	Task<HttpContent?> GetAppleWalletCard([Authorize("Bearer")] string bearerToken, [AliasAs("patronId")] string? patronId);

	[Get("/api/mobile/1.0/patrons/{patronId}/wallet/google?language={isoCode}")]
	Task<IApiResponse<string>> GetGoogleWalletCard([Authorize("Bearer")] string bearerToken, [AliasAs("patronId")] string? patronId, [AliasAs("isoCode")] string? isoCode);

	[Get("/api/mobile/1.0/mobile/branches?setId={setId}&lastIrn={lastIrn}&count={count}")]
	Task<IApiResponse<BranchResponseDto>> GetBranches([AliasAs("setId")] long setId, [AliasAs("lastIrn")] long lastIrn, [AliasAs("count")] int count, [Header("spydus-lang")] string? languageIdentifier);
}
