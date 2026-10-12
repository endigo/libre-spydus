using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class ContainerDto
{
	public string? Title { get; set; }

	public long? LoginRequired { get; set; }

	public string? Icon { get; set; }

	public string? Url { get; set; }

	public string? Type { get; set; }

	[JsonIgnore]
	public bool IsExternalLink { get; set; }

	[JsonIgnore]
	public bool OnlyDisplayWhenLoggedIn => LoginRequired == 1;

	[JsonIgnore]
	public bool OnlyDisplayWhenNotLoggedIn => LoginRequired == 2;
}
