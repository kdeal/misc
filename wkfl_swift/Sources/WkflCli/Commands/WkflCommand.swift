import ArgumentParser

struct WkflCommand: ParsableCommand {
  static let configuration = CommandConfiguration(
    commandName: "wkfl",
    version: "0.1.4"
  )

  @Flag(name: .shortAndLong, help: "Enable verbose (debug) logging output")
  var verbose = false

  @Option(name: .long, help: "Write generated shell integration commands to this file")
  var shellActionsFile: String?

  mutating func run() throws {
    throw ValidationError("Missing subcommand.")
  }
}
