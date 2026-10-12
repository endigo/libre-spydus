namespace Spydus.Domain.DTO;

public class SetInfo
{
	public long SetId { get; set; }

	public long LastIrn { get; set; }

	public int SetTotal { get; set; }

	public int SetCount { get; set; }

	public bool HasMoreRecords { get; set; }
}
