using System;
using System.Text.Json.Serialization;
using Microsoft.Maui.Controls;

namespace Spydus.Domain.DTO;

public class AnnouncementDto
{
	public string? Title { get; set; }

	public DateTime? StartDate { get; set; }

	public DateTime? EndDate { get; set; }

	public string? Content { get; set; }

	[JsonIgnore]
	public HtmlWebViewSource HtmlSource => new HtmlWebViewSource
	{
		Html = $"<!DOCTYPE html>\n\t\t\t\t<html>\n\t\t\t\t<head>\n\t\t\t\t    <meta charset=\"UTF-8\">\n\t\t\t\t    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no\">\n\t\t\t\t    <style>\n\t\t\t\t        html, body {{\n\t\t\t\t            margin: 0;\n\t\t\t\t            padding: 0;\n\t\t\t\t            width: 100%;\n\t\t\t\t            box-sizing: border-box;\n\t\t\t\t            overflow-x: hidden;\n\t\t\t\t        }}\n\n\t\t\t\t        *, *::before, *::after {{\n\t\t\t\t            box-sizing: inherit;\n\t\t\t\t        }}\n\n\t\t\t\t        p {{\n\t\t\t\t            margin: 0 0 12px 0;\n\t\t\t\t            font-size: 20px;\n\t\t\t\t            font-family: Arial, sans-serif;\n\t\t\t\t            line-height: 1.4;\n\t\t\t\t            word-wrap: break-word;\n\t\t\t\t            overflow-wrap: break-word;\n\t\t\t\t        }}\n\n\t\t\t\t        p:last-child {{\n\t\t\t\t            margin-bottom: 0;\n\t\t\t\t        }}\n\t\t\t\t    </style>\n\t\t\t\t</head>\n\t\t\t\t<body>\n\t\t\t\t    <p style=\"color: #262626;\"><b>{Title}</b></p>\n\t\t\t\t    <p style=\"color: #65666A;\">{Content}</p>\n\t\t\t\t</body>\n\t\t\t\t</html>"
	};
}
