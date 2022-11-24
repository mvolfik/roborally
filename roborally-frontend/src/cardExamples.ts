export const cardExamples = [
  {
    name: "Move forward until you hit a wall or a hole (without falling into it)",
    code: `fn execute(player_i, register_i) {
  let dir = GAME.get_player_direction(player_i);
  let can_move = !GAME.is_void_at(GAME.get_player_position(player_i) + dir);
  while can_move {
    let res = GAME.move_player_in_direction(player_i, dir);
    can_move = res.moved && !GAME.is_void_at(GAME.get_player_position(player_i) + dir);
  }
}`,
  },
  {
    name: "Jump onto the nearest player (in any direction)",
    code: `fn find_closest_player_pos(player_pos, player_dir, GAME) {
  print("Player at " + player_pos.x + "," + player_pos.y + " seeking jump target\\n");
  let dist = 0;
  loop {
    dist += 1;
    let pos = player_pos;
    for i in 0..dist {
      pos = pos + player_dir;
    }
    let clockwise_dir = player_dir;
    for i_ in 0..4 {
      clockwise_dir = clockwise_dir + 1;
      for j_ in 0..dist {
        print("Checking " + pos.x + "," + pos.y + "\\n");
        let found_player = GAME.get_player_at_position(pos);
        if found_player != () {
          print("Found player at that tile: player no. " + (found_player + 1) + "\\n");
          return pos;
        }
        pos = (pos + clockwise_dir) + (clockwise_dir + 1);
      }
    }
  }
}

fn execute(player_i, register_i) {
  if PLAYER_COUNT <= 1 {
    print("Can't use this card if there's only one player!\\n");
  }
  let player_pos = GAME.get_player_position(player_i);
  let player_dir = GAME.get_player_direction(player_i);
  let closest_player_pos = find_closest_player_pos(player_pos, player_dir, GAME);
  GAME.force_move_player_to(player_i, closest_player_pos, player_dir);
}`,
  },
];
