using System.Linq;
using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class AppLanguageDto
{
	[JsonPropertyName("iso-code")]
	public string? IsoCode { get; set; }

	[JsonPropertyName("language-pac")]
	public string? LanguagePac { get; set; }

	public string? Name { get; set; }

	public string? LanguageName => IsoCode?.Split('-')?.FirstOrDefault();
}
