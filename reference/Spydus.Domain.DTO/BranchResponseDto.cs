using System.Collections.Generic;

namespace Spydus.Domain.DTO;

public class BranchResponseDto
{
	public SetInfo SetInfo { get; set; }

	public List<BranchDto>? Branches { get; set; }
}
