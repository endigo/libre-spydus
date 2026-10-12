using System;
using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class NotificationDto
{
	[JsonPropertyName("brwdIrn")]
	public long BorrowerConsortiumRecordIdentifier { get; set; }

	[JsonPropertyName("brwIrn")]
	public long BorrowerLocalAccountIdentifier { get; set; }

	[JsonPropertyName("createdDate")]
	public DateTimeOffset CreatedDate { get; set; }

	[JsonPropertyName("ntfId")]
	public Guid Id { get; set; }

	[JsonPropertyName("instCode")]
	public string? InstCode { get; set; }

	[JsonPropertyName("isRead")]
	public bool IsRead { get; set; }

	[JsonPropertyName("libraryId")]
	public string? LibraryId { get; set; }

	[JsonPropertyName("message")]
	public string? Message { get; set; }

	[JsonPropertyName("subject")]
	public string? Subject { get; set; }

	[JsonPropertyName("url")]
	public string? Url { get; set; }
}
