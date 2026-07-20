import ArgumentParser

@main
struct wkfl: ParsableCommand {
    @Flag(name: .shortAndLong, help: "Enable verbose (debug) logging output")
    var verbose = false

    public func run() throws {
        print("Hello, world!")
    }
}
