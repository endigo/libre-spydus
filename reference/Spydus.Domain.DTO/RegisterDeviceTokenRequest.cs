using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class RegisterDeviceTokenRequest
{
	[JsonPropertyName("deviceToken")]
	public string? DeviceToken { get; set; }

	[JsonPropertyName("platform")]
	public string? Platform { get; set; }

	[JsonPropertyName("tags")]
	public List<string>? Tags { get; set; }
}
