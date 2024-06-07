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
	gh auth status
	git cliff --strip=all --unreleased
	@read -p 'version> ' TAG && \
		$(SED) "s|^version = .*|version = \"$$TAG\"|" -i Cargo.toml && \
		$(CARGO) build --release && \
		git cliff --tag "v$$TAG" > CHANGELOG.md && \
		$(GIT) commit --gpg-sign --message "chore(release): version v$$TAG" Cargo.toml Cargo.lock CHANGELOG.md && \
		$(GIT) tag --sign --message "Version v$$TAG" v$$TAG && \
		$(GIT) archive -o $(TARBALLDIR)/git-smash-v$$TAG.$(TARBALLFORMAT) --format $(TARBALLFORMAT) --prefix=git-smash-$$TAG/ v$$TAG && \
		$(GPG) --detach-sign $(TARBALLDIR)/git-smash-v$$TAG.$(TARBALLFORMAT) && \
		git push origin main && \
		git push origin v$$TAG && \
		gh release create \
			--title "v$$TAG" \
			--notes-file <(git cliff --strip=all --latest) \
			"v$$TAG" \
			git-smash-v$$TAG.$(TARBALLFORMAT) \
			git-smash-v$$TAG.$(TARBALLFORMAT).sig && \
		cargo publish
