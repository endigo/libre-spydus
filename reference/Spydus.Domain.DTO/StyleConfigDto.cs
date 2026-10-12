using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class StyleConfigDto
{
	[JsonPropertyName("barcodeType")]
	public string? BarcodeType { get; set; }

	public string? BranchUrl { get; set; }

	[JsonPropertyName("contentContainerUrl")]
	public string? ContentContainerUrl { get; set; }

	[JsonPropertyName("langs")]
	public List<LanguageDto>? Langs { get; set; }

	[JsonPropertyName("logoUrl")]
	public string? LogoUrl { get; set; }

	[JsonPropertyName("notificationHubConnectionString")]
	public string? NotificationHubConnectionString { get; set; }

	[JsonPropertyName("notificationHubName")]
	public string? NotificationHubName { get; set; }

	[JsonPropertyName("selfIssueEnabled")]
	public bool? SelfIssueEnabled { get; set; }

	[JsonPropertyName("selfReturnEnabled")]
	public bool? SelfReturnEnabled { get; set; }

	[JsonPropertyName("enableJoinOnline")]
	public bool? EnableJoinOnline { get; set; }

	[JsonPropertyName("spydusViewOverride")]
	public string? SpydusViewOverride { get; set; }

	[JsonPropertyName("style")]
	public Dictionary<string, string>? Style { get; set; }

	[JsonPropertyName("wifi")]
	public Wifi? Wifi { get; set; }

	public DiscoveryDto? Discovery { get; set; }

	public string? JoinOnlineIcon { get; set; }

	public string? DistanceUnit { get; set; }

	public string? LibraryAppTimeZone { get; set; }

	public bool? ShowNearbyBranches { get; set; }
}
