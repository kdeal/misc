# Why

## Why stderr for prompts

We use stderr because it allows you to pipe the output of the command to another
command. If the prompt was also printed to stdout then it would get forwarded to
the program and the user wouldn't see it.
