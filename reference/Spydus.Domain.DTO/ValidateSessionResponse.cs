using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class ValidateSessionResponse
{
	[JsonPropertyName("institutionId")]
	public string? InstitutionId { get; set; }

	[JsonPropertyName("name")]
	public string? Name { get; set; }

	[JsonPropertyName("barcode")]
	public string? Barcode { get; set; }

	[JsonPropertyName("brwIrn")]
	public string? BorrowerLocalAccountIdentifier { get; set; }

	[JsonPropertyName("brwdIrn")]
	public string? BorrowerConsortiumRecordIdentifier { get; set; }

	[JsonPropertyName("brwSessionId")]
	public string? BorrowerSessionId { get; set; }
}
