using System.Threading.Tasks;
using Refit;
using Spydus.Domain.DTO;

namespace Spydus.Domain.Interface;

public interface IValidateSessionClient
{
	[Post("/api/private/1.0/patrons/validatesessionid")]
	[Headers(new string[] { "Content-Type: application/json; charset=UTF-8" })]
	Task<IApiResponse<ValidateSessionResponse>> ValidateSessionId([Body] ValidateSessionRequest request);
}
