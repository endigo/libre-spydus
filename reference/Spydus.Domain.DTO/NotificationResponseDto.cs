using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class NotificationResponseDto
{
	[JsonPropertyName("notifications")]
	public List<NotificationDto>? Notifications { get; set; }

	[JsonPropertyName("totalCount")]
	public long TotalCount { get; set; }
}
