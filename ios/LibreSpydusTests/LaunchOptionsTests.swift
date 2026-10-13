import Testing
@testable import LibreSpydus

struct LaunchOptionsTests {
    @Test func emptyWithoutHooks() {
        #expect(LaunchOptions.parse(["/path/LibreSpydus"]) == LaunchOptions())
        #expect(LaunchOptions.parse(["/path/LibreSpydus", "-NSDoubleLocalizedStrings", "YES"]) == LaunchOptions())
    }

    @Test func parsesTheDocumentedExample() {
        let options = LaunchOptions.parse([
            "app", "-LSLibraryOrigin", "https://canning.spydus.com", "-LSDemo", "YES", "-LSRoute", "borrowed",
        ])
        #expect(options.libraryOrigin == "https://canning.spydus.com")
        #expect(options.demo)
        #expect(!options.reset)
        #expect(options.route == .borrowed)
    }

    @Test(arguments: [
        ("home", LaunchOptions.LaunchRoute.home),
        ("search:harry potter", .search("harry potter")),
        ("search", .search("")),
        ("results:new books", .results("new books")),
        ("detail:https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=1",
         .detail("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=1")),
        ("borrowed", .borrowed),
        ("favourites", .favourites),
        ("profile", .profile),
        ("libraries", .libraries),
        ("login", .login),
        ("verify", .verify),
        ("gallery", .gallery),
    ])
    func parsesEveryRoute(text: String, expected: LaunchOptions.LaunchRoute) {
        #expect(LaunchOptions.parse(["app", "-LSRoute", text]).route == expected)
    }

    @Test func rejectsUnknownOrIncompleteRoutes() {
        #expect(LaunchOptions.LaunchRoute("nope") == nil)
        #expect(LaunchOptions.LaunchRoute("detail:") == nil)
        #expect(LaunchOptions.LaunchRoute("results") == nil)
    }

    @Test func booleanFlags() {
        #expect(LaunchOptions.parse(["app", "-LSDemo", "true", "-LSReset", "1"]).demo)
        #expect(LaunchOptions.parse(["app", "-LSDemo", "true", "-LSReset", "1"]).reset)
        #expect(!LaunchOptions.parse(["app", "-LSDemo", "NO"]).demo)
        #expect(!LaunchOptions.parse(["app", "-LSDemo"]).demo)
    }

    @MainActor
    @Test func appliesRoutesToTheModel() {
        let model = AppModel(dependencies: .testing())
        LaunchOptions.parse(["app", "-LSLibraryOrigin", "https://canning.spydus.com", "-LSRoute", "results:cats"])
            .apply(to: model)
        #expect(model.library?.origin == "https://canning.spydus.com")
        #expect(model.selectedTab == .search)
        #expect(model.path(for: .search) == [.results(query: "cats", sort: .relevance, title: "cats")])

        LaunchOptions.parse(["app", "-LSRoute", "verify"]).apply(to: model)
        #expect(model.rootSheet == .verification)
    }
}
