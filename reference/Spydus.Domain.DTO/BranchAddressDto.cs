using System.Text;

namespace Spydus.Domain.DTO;

public class BranchAddressDto
{
	public string NumberAndStreet { get; set; }

	public string StreetLine2 { get; set; }

	public string StreetLine3 { get; set; }

	public string Suburb { get; set; }

	public string Postcode { get; set; }

	public string State { get; set; }

	public string Country { get; set; }

	public string FullAddress => GetFullAddress();

	private string GetFullAddress()
	{
		StringBuilder stringBuilder = new StringBuilder();
		if (!string.IsNullOrWhiteSpace(NumberAndStreet))
		{
			stringBuilder.Append(NumberAndStreet);
			stringBuilder.Append(", ");
		}
		if (!string.IsNullOrWhiteSpace(StreetLine2))
		{
			stringBuilder.Append(StreetLine2);
			stringBuilder.Append(", ");
		}
		if (!string.IsNullOrWhiteSpace(StreetLine3))
		{
			stringBuilder.Append(StreetLine3);
			stringBuilder.Append(", ");
		}
		if (!string.IsNullOrWhiteSpace(Suburb))
		{
			stringBuilder.Append(Suburb);
			stringBuilder.Append(", ");
		}
		if (!string.IsNullOrWhiteSpace(Postcode))
		{
			stringBuilder.Append(Postcode);
			stringBuilder.Append(", ");
		}
		if (!string.IsNullOrWhiteSpace(State))
		{
			stringBuilder.Append(State);
			stringBuilder.Append(", ");
		}
		if (!string.IsNullOrWhiteSpace(Country))
		{
			stringBuilder.Append(Country);
			stringBuilder.Append(", ");
		}
		return stringBuilder.ToString().TrimEnd().TrimEnd(',');
	}
}
