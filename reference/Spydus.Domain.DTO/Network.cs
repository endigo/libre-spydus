using System.Collections.Generic;

namespace Spydus.Domain.DTO;

public class Network
{
	public List<string>? Actions { get; set; }

	public string? Description { get; set; }

	public string? Location { get; set; }

	public string? Name { get; set; }
}
