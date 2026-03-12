group "default" {
  targets = ["ocelot"]
}

target "ocelot" {
  dockerfile = "dev-support/containers/alpine/Containerfile"
  platforms  = ["linux/amd64"]
  target     = "ocelot"
  contexts = {
    rust   = "docker-image://docker.io/library/rust:1.94.0-alpine3.23"
    alpine = "docker-image://docker.io/library/alpine:3.23"
  }
  args = {
    RUSTC_WRAPPER         = "/usr/bin/sccache"
    SCCACHE_GHA_ENABLED   = "off"
    ACTIONS_CACHE_URL     = null
    ACTIONS_RUNTIME_TOKEN = null
  }
  labels = {
    "description"                     = "Process supervisor and init system written in Rust Programming Language"
    "image.type"                      = "final"
    "image.authors"                   = "46590321+xrelkd@users.noreply.github.com"
    "image.vendor"                    = "xrelkd"
    "image.description"               = "Process supervisor and init system written in Rust Programming Language"
    "org.opencontainers.image.source" = "https://github.com/xrelkd/ocelot"
  }
}
