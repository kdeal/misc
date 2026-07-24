import Testing
@testable import WkflCli

@Test func rootCommandUsesExecutableName() {
    #expect(WkflCommand.configuration.commandName == "wkfl")
}

@Test func verboseFlagParses() throws {
    let command = try WkflCommand.parse(["--verbose"])

    #expect(command.verbose)
}
