using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class ValidateSessionRequest
{
	[JsonPropertyName("sessionId")]
	public string? BorrowerSessionId { get; set; }
}
