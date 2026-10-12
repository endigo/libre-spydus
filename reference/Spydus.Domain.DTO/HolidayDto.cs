using System.Text.Json.Serialization;

namespace Spydus.Domain.DTO;

public class HolidayDto
{
	public string StartDate { get; set; }

	public string EndDate { get; set; }

	public string Name { get; set; }

	[JsonIgnore]
	public string StartDateTime { get; set; }

	[JsonIgnore]
	public string EndDateTime { get; set; }

	[JsonIgnore]
	public string DisplayDateRange
	{
		get
		{
			if (!(StartDateTime == EndDateTime))
			{
				return StartDateTime + " - " + EndDateTime;
			}
			return StartDateTime;
		}
	}
}
