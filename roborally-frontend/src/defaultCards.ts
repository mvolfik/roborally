export const DEFAULT_CARDS = {
  again_count: 2,
  cards: [
    {
      assetName: "turn-right",
      name: "Turn Right",
      code: [
        "GAME.set_player_direction(player_i, GAME.get_player_direction(player_i) + 1)",
      ],
      count: 3,
    },
    {
      assetName: "turn-left",
      name: "Turn Left",
      code: [
        "GAME.set_player_direction(player_i, GAME.get_player_direction(player_i) - 1)",
      ],
      count: 3,
    },
    {
      assetName: "u-turn",
      name: "U-Turn",
      code: Array(2).fill(
        "GAME.set_player_direction(player_i, GAME.get_player_direction(player_i) - 1)"
      ),
      count: 1,
    },
    {
      assetName: "move1",
      name: "Move 1",
      code: [
        "GAME.move_player_in_direction(player_i, GAME.get_player_direction(player_i))",
      ],
      count: 5,
    },
    {
      assetName: "move2",
      name: "Move 2",
      code: Array(2).fill(
        "GAME.move_player_in_direction(player_i, GAME.get_player_direction(player_i))"
      ),
      count: 3,
    },
    {
      assetName: "move3",
      name: "Move 3",
      code: Array(3).fill(
        "GAME.move_player_in_direction(player_i, GAME.get_player_direction(player_i))"
      ),
      count: 1,
    },
    {
      assetName: "reverse1",
      name: "Reverse 1",
      code: [
        "GAME.move_player_in_direction(player_i, GAME.get_player_direction(player_i) + 2)",
      ],
      count: 2,
    },
  ].map(({ assetName, code, ...rest }) => ({
    asset: new URL(`/assets/${assetName}.png`, window.location.href).toString(),
    code: [
      "fn execute(player_i, register_i) {",
      ...code.map((l) => "  " + l + ";"),
      "}",
    ].join("\n"),
    ...rest,
  })),
};

export const NEW_CARD = {
  code: "",
  asset: "https://example.com/image.png",
  count: 1,
  name: "New Card",
};
