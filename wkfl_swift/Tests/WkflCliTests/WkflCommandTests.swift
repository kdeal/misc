import Testing

@testable import WkflCli

@Test func rootCommandUsesExecutableName() {
  #expect(WkflCommand.configuration.commandName == "wkfl")
}

@Test func verboseFlagParses() throws {
  let command = try WkflCommand.parse(["--verbose"])

  #expect(command.verbose)
}

@Test func shellActionsFileOptionParses() throws {
  let command = try WkflCommand.parse(["--shell-actions-file", "actions.txt"])

  #expect(command.shellActionsFile == "actions.txt")
}

@Test func versionWritesToInjectedStandardOutput() {
  var standardOutput = ""
  var standardError = ""

  let status = WkflCli.run(
    arguments: ["--version"],
    standardOutput: &standardOutput,
    standardError: &standardError
  )

  #expect(status == 0)
  #expect(standardOutput == "0.1.4\n")
  #expect(standardError.isEmpty)
}

@Test func shortVersionWritesToInjectedStandardOutput() {
  var standardOutput = ""
  var standardError = ""

  let status = WkflCli.run(
    arguments: ["-V"],
    standardOutput: &standardOutput,
    standardError: &standardError
  )

  #expect(status == 0)
  #expect(standardOutput == "0.1.4\n")
  #expect(standardError.isEmpty)
}

@Test func usageErrorsWriteToInjectedStandardError() {
  var standardOutput = ""
  var standardError = ""

  let status = WkflCli.run(
    arguments: ["--not-an-option"],
    standardOutput: &standardOutput,
    standardError: &standardError
  )

  #expect(status == 2)
  #expect(standardOutput.isEmpty)
  #expect(standardError.contains("Unknown option '--not-an-option'"))
}

@Test func rootWithoutSubcommandIsAUsageError() {
  var standardOutput = ""
  var standardError = ""

  let status = WkflCli.run(
    arguments: [],
    standardOutput: &standardOutput,
    standardError: &standardError
  )

  #expect(status == 2)
  #expect(standardOutput.isEmpty)
  #expect(standardError.contains("Missing subcommand."))
}
