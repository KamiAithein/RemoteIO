c-bind:
	ghc -c -O src/Lib.hs -odir c-bind -hidir c-bind -stubdir c-bind
	cp src/Lib.hs c-bind/Lib.hs

c-app: c-bind c-bind/test.c
	cd c-bind && ghc --make -no-hs-main -optc-O test.c Lib -o test

.PHONY: clean c-bind

clean:
	rm c-bind/test
	rm c-bind/Lib*
	rm c-bind/*.o
