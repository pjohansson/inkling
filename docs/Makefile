TARGET=../target/debug
TMPDEPLOY=/tmp/book

.PHONY: build
build:
	@echo "==> compiling book"
	mdbook build
	@echo ""

test: compile
	@echo "==> testing code examples in user guide"
	mdbook test --library-path ${TARGET}
	@echo ""

.PHONY: compile
compile:
	@echo "==> compiling debug build of \`inkling\` in ${TARGET}"
	cargo build
	@echo ""

.PHONY: deploy
deploy: build
	@echo "==> deploying to github"
	git worktree add ${TMPDEPLOY} gh-pages
	rm -rf ${TMPDEPLOY}/*
	cp -rp book/* ${TMPDEPLOY}/
	cd ${TMPDEPLOY} && \
		git add -A && \
		git commit -m "deployed on $(shell date) by ${USER}" && \
		git push origin gh-pages
	git worktree remove ${TMPDEPLOY}
	@echo ""