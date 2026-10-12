using System.Threading.Tasks;
using Refit;
using Spydus.Domain.DTO.Auth;

namespace Spydus.Domain.Interface;

public interface IAuthClient
{
	[Get("/openid/refreshtoken")]
	Task<IApiResponse<AccessTokenResponse>> GetAccessToken([AliasAs("refreshToken")] string refreshToken, [AliasAs("clientId")] string clientId);

	[Get("/openid/token")]
	Task<IApiResponse<RefreshTokenResponse>> GetRefreshToken([AliasAs("code")] string authorizationCode, [AliasAs("grant_type")] string grantType, [AliasAs("client_id")] string clientId);
}
