# roborally

Web version of the RoboRally board game – high school graduation project

[Rules download link](https://www.hasbro.com/common/documents/60D52426B94D40B98A9E78EE4DD8BF94/3EA9626BCAE94683B6184BD7EA3F1779.pdf)

## Build/release

```sh
git archive --prefix roborally-mvolf/ -o source-code.tar.gz master && \
git archive --prefix roborally-mvolf/ -o source-code.zip master && \
env DOCKER_BUILDKIT=1 docker build -t roborally:dev . && \
docker run --rm -p 80:80 -e PORT=80 roborally:dev
```

## Architektura

TODO
