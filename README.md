# roborally

Web version of the RoboRally board game – high school graduation project

[Rules download link](https://www.hasbro.com/common/documents/60D52426B94D40B98A9E78EE4DD8BF94/3EA9626BCAE94683B6184BD7EA3F1779.pdf)

## Build/release

```sh
git archive -o source-code.tar.gz master && env DOCKER_BUILDKIT=1 docker build -t roborally:dev .
docker run --rm -p 80:80 -e PORT=80 roborally:dev
```
## Rule differences

- no energy cubes & powerups
- no board lasers (yet?)
- 1 reboot token for whole map
- spawn points are assigned randomly (no player choice)
- running out of SPAM cards isn't supported (yet) (no player choice)
- reboot token has set orientation (no player choice)
- belt movements also sorted by priority antenna (no move-to-same-tile edge-case)
