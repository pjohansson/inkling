TARGET=../target/debug
TMPDEPLOY=/tmp/book

test: build
	mdbook test --library-path ${TARGET}

.PHONY: build
build:
	cargo build

.PHONY: deploy
deploy: book
	@echo "==> deploying to github"
	git worktree add ${TMPDEPLOY} gh-pages
	rm -rf ${TMPDEPLOY}/*
	cp -rp book/* ${TMPDEPLOY}/
	cd ${TMPDEPLOY} && \
		git add -A && \
		git commit -m "deployed on $(shell date) by ${USER}" && \
		git push origin gh-pages
		git worktree remove ${TMPDEPLOY}