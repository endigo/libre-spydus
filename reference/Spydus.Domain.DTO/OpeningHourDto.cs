using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class OpeningHourDto
{
	public object Date { get; set; }

	public int Dow { get; set; }

	public string DayName { get; set; }

	public string OpenTime { get; set; }

	public string CloseTime { get; set; }

	public bool IsOpen { get; set; }

	[JsonIgnore]
	public string DisplayTime { get; set; }

	[JsonIgnore]
	public bool ShouldHighlightToday { get; set; }
}
