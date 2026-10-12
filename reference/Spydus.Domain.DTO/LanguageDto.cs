using System.Linq;
using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class LanguageDto
{
	[JsonPropertyName("desc")]
	public string? Desc { get; set; }

	[JsonPropertyName("id")]
	public string? Id { get; set; }

	[JsonPropertyName("iso")]
	public string? ISO { get; set; }

	public string? LanguageName => ISO?.Split('-').FirstOrDefault();
}
