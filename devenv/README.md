# devenv
This is a docker container with some basic tools for doing development work.
Right now it has tooling for running neovim and doing python or javascript
development. This exposes an ssh server running on port 22 and you must set
`SSH_USER` and `SSH_PUB_KEY` environment variables, so that you can ssh into the
container to work from it.
