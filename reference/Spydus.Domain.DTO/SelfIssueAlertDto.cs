namespace Spydus.Domain.DTO;

public class SelfIssueAlertDto
{
	public string? Code { get; set; }

	public bool Critical { get; set; }

	public int Irn { get; set; }

	public string? Ref { get; set; }

	public string? Text { get; set; }
}
