using System.Collections.Generic;

namespace Spydus.Domain.DTO;

public class BranchDto
{
	public int Irn { get; set; }

	public string Id { get; set; }

	public string Name { get; set; }

	public double? Latitude { get; set; }

	public double? Longitude { get; set; }

	public string Url { get; set; }

	public string OpacUrl { get; set; }

	public string ImageUrl { get; set; }

	public string Phone { get; set; }

	public string Email { get; set; }

	public BranchAddressDto? Address { get; set; }

	public List<OpeningHourDto>? OpeningHours { get; set; }

	public List<HolidayDto>? Holidays { get; set; }
}
