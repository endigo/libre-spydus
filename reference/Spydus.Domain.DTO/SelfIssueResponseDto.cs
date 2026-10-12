using System;
using System.Collections.Generic;

namespace Spydus.Domain.DTO;

public class SelfIssueResponseDto
{
	public List<SelfIssueAlertDto>? Alerts { get; set; }

	public string? BibAuthor { get; set; }

	public string? BibIssueTitle { get; set; }

	public string? BibTitle { get; set; }

	public string? CoverImage { get; set; }

	public DateTime? DueDate { get; set; }

	public bool? IsIssued { get; set; }

	public bool? ItemCheckedIn { get; set; }

	public string? ItemId { get; set; }

	public int? ItemIrn { get; set; }

	public string? ItemRfid { get; set; }

	public int? LoanIrn { get; set; }

	public int? PatronIrn { get; set; }

	public double? RentalAmount { get; set; }
}
