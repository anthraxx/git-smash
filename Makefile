-include Makefile.local

DESTDIR ?=
PREFIX ?= /usr/local
BINDIR ?= ${PREFIX}/bin
DATAROOTDIR ?= ${PREFIX}/share
MANDIR ?= ${DATAROOTDIR}/man

TARBALLDIR ?= target/release/tarball
TARBALLFORMAT=tar.gz

RM := rm
CARGO := cargo
SCDOC := scdoc
INSTALL := install
GIT := git
GPG := gpg
SED := sed

DEBUG := 0
ifeq ($(DEBUG), 0)
	CARGO_OPTIONS := --release --locked
	CARGO_TARGET := release
else
	CARGO_OPTIONS :=
	CARGO_TARGET := debug
endif

.PHONY: all git-smash test docs completions clean install uninstall

all: git-smash test docs

git-smash:
	$(CARGO) build $(CARGO_OPTIONS)

test:
	$(CARGO) test $(CARGO_OPTIONS)

lint:
	$(CARGO) fmt -- --check
	$(CARGO) check
	find . -name '*.rs' -exec touch {} +
	$(CARGO) clippy --all -- -D warnings

docs: completions

completions: git-smash
	target/$(CARGO_TARGET)/git-smash completions bash | $(INSTALL) -Dm 644 /dev/stdin target/completion/bash/git-smash
	target/$(CARGO_TARGET)/git-smash completions zsh | $(INSTALL) -Dm 644 /dev/stdin target/completion/zsh/_git-smash
	target/$(CARGO_TARGET)/git-smash completions fish | $(INSTALL) -Dm 644 /dev/stdin target/completion/fish/git-smash.fish

clean:
	$(RM) -rf target contrib/man/*.1

install: git-smash docs
	$(INSTALL) -Dm 755 target/$(CARGO_TARGET)/git-smash -t $(DESTDIR)$(BINDIR)
	$(INSTALL) -Dm 644 target/completion/bash/git-smash -t $(DESTDIR)$(DATAROOTDIR)/bash-completion/completions
	$(INSTALL) -Dm 644 target/completion/zsh/_git-smash -t  $(DESTDIR)$(DATAROOTDIR)/zsh/site-functions
	$(INSTALL) -Dm 644 target/completion/fish/git-smash.fish -t  $(DESTDIR)$(DATAROOTDIR)/fish/vendor_completions.d

uninstall:
	$(RM) -f $(DESTDIR)$(BINDIR)/git-smash
	$(RM) -f $(DESTDIR)$(DATAROOTDIR)/bash-completion/completions/git-smash
	$(RM) -f $(DESTDIR)$(DATAROOTDIR)/zsh/site-functions/_git-smash
	$(RM) -f $(DESTDIR)$(DATAROOTDIR)/fish/vendor_completions.d/git-smash.fish

release: all
	$(INSTALL) -d $(TARBALLDIR)
	@read -p 'version> ' TAG && \
		$(SED) "s|version = .*|version = \"$$TAG\"|" -i Cargo.toml && \
		$(CARGO) build --release && \
		$(GIT) commit --gpg-sign --message "version: release $$TAG" Cargo.toml Cargo.lock && \
		$(GIT) tag --sign --message "version: release $$TAG" $$TAG && \
		$(GIT) archive -o $(TARBALLDIR)/git-smash-$$TAG.$(TARBALLFORMAT) --format $(TARBALLFORMAT) --prefix=git-smash-$$TAG/ $$TAG && \
		$(GPG) --detach-sign $(TARBALLDIR)/git-smash-$$TAG.$(TARBALLFORMAT)
