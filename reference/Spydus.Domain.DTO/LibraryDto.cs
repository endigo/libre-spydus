using System;
using Spydus.Abstractions.Model;

namespace Spydus.Domain.DTO;

public class LibraryDto : ILibrary
{
	public string? ActivationCode { get; set; }

	public Guid Id { get; set; }

	public LocationDto? Location { get; set; }

	public string? Logo { get; set; }

	public string? Name { get; set; }

	public string[]? PinningHash { get; set; }

	public string? Url { get; set; }
}
