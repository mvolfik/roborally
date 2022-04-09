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

## Rule differences

- no energy cubes & powerups
- 1 reboot token for whole map
  - it wouldn't be that hard to implement multiple reboot tokens, where each of them will have a
    specified rectangle where it is active (and checkpoints will be a fallback). The biggest issue
    I currently see with this is how to indicate that in the frontend
- spawn points are assigned randomly (no player choice)
- only 1 type of damage cards, running out of SPAM cards isn't supported (yet) (no player choice)
- reboot token has set orientation (no player choice)
  - a drawback of this is that there's now a risk of entering an infinite reboot cycle - we panic
    in that case
- belt movements also sorted by priority antenna (no move-to-same-tile edge-case) - this needs to be fixed, since it also introduces some other weird behaviors
- programming Again after damage card re-executes the substitute action,
  doesn't draw another card (why would anyone ever program cards like that anyway)
- board lasers are always only 1-hit

## TODOs

- animations: reboot, player move attempt
- belts movement
- remove special init message
- fix: player on reboot token -> you push him out of map -> two players suddenly on 1 tile

## Possible upgrades in the future

- all moving phase state is generated and sent from the server in one batch, and the client
  can play it at their own chosen speed, including the option of manual stepping
