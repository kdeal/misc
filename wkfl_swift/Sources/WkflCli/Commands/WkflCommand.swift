import ArgumentParser

struct WkflCommand: ParsableCommand {
    static let configuration = CommandConfiguration(commandName: "wkfl")

    @Flag(name: .shortAndLong, help: "Enable verbose (debug) logging output")
    var verbose = false
}
