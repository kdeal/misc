import ArgumentParser
import Foundation

#if os(Linux)
  import Glibc
#else
  import Darwin
#endif

/// The runtime state supplied only to commands that require configuration or shell actions.
struct WkflRuntimeContext {
  let verbose: Bool
  let shellActionsFile: String?
}

/// Commands that need runtime state opt in so help, completion, and TUI-only commands stay independent.
protocol WkflRuntimeCommand: ParsableCommand {
  mutating func run(using context: WkflRuntimeContext) throws
}

enum WkflCli {
  @discardableResult
  static func run<StandardOutput: TextOutputStream, StandardError: TextOutputStream>(
    arguments: [String],
    standardOutput: inout StandardOutput,
    standardError: inout StandardError,
    makeContext: (WkflRuntimeContext, any ParsableCommand) throws -> WkflRuntimeContext = {
      context, _ in context
    }
  ) -> Int32 {
    let command: any ParsableCommand

    do {
      command = try WkflCommand.parseAsRoot(arguments.map { $0 == "-V" ? "--version" : $0 })
    } catch {
      let parserExitCode = WkflCommand.exitCode(for: error).rawValue
      let exitCode: Int32 = parserExitCode == 0 ? 0 : 2
      let message = WkflCommand.fullMessage(for: error)
      if exitCode == 0 {
        write(message, to: &standardOutput)
      } else {
        write(message, to: &standardError)
      }
      return exitCode
    }

    do {
      if var runtimeCommand = command as? any WkflRuntimeCommand {
        let rootCommand = command as? WkflCommand
        let context = WkflRuntimeContext(
          verbose: rootCommand?.verbose ?? false,
          shellActionsFile: rootCommand?.shellActionsFile
        )
        try runtimeCommand.run(using: makeContext(context, command))
      } else {
        var command = command
        try command.run()
      }
      return 0
    } catch {
      let exitCode: Int32 = error is ValidationError ? 2 : WkflCommand.exitCode(for: error).rawValue
      let message = WkflCommand.fullMessage(for: error)
      if exitCode == 0 {
        write(message, to: &standardOutput)
      } else {
        write(message, to: &standardError)
      }
      return exitCode
    }
  }

  private static func write<Stream: TextOutputStream>(_ message: String, to stream: inout Stream) {
    guard !message.isEmpty else { return }
    stream.write(message)
    if !message.hasSuffix("\n") {
      stream.write("\n")
    }
  }
}

private struct FileHandleOutputStream: TextOutputStream {
  let handle: FileHandle

  mutating func write(_ string: String) {
    handle.write(Data(string.utf8))
  }
}

@main
enum WkflCLI {
  static func main() {
    var standardOutput = FileHandleOutputStream(handle: .standardOutput)
    var standardError = FileHandleOutputStream(handle: .standardError)
    let status = WkflCli.run(
      arguments: Array(CommandLine.arguments.dropFirst()),
      standardOutput: &standardOutput,
      standardError: &standardError
    )
    exit(status)
  }
}
