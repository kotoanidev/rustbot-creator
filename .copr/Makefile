# Copr's "make srpm" source method (D-144). Copr runs, in a clone of the repository:
#   make -f .copr/Makefile srpm outdir=<folder> spec=<spec>
# It makes a source tarball of the checkout, then the SRPM from it.

SPEC := packaging/rpm/rustbot-creator.spec

srpm:
	dnf -y install git rpm-build
	git config --global --add safe.directory '*'
	version=$$(sed -n 's/^Version: *//p' $(SPEC)); \
	git archive --format=tar.gz --prefix=rustbot-creator-$$version/ \
		-o "$(outdir)/rustbot-creator-$$version.tar.gz" HEAD && \
	rpmbuild -bs $(SPEC) --define "_sourcedir $(outdir)" --define "_srcrpmdir $(outdir)"
