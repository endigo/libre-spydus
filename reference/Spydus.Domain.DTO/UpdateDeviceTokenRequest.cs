using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class UpdateDeviceTokenRequest
{
	[JsonPropertyName("newDeviceToken")]
	public string? NewDeviceToken { get; set; }

	[JsonPropertyName("oldDeviceToken")]
	public string? OldDeviceToken { get; set; }

	[JsonPropertyName("platform")]
	public string? Platform { get; set; }
}
