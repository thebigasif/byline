/** One emoji the writer can add by its name. */
export type Emoji = { name: string; emoji: string };

// The names GitHub and Slack use, so a writer's habits carry over. Pairs of
// a name and its emoji, read once below.
const TABLE = `
smile 😄 grinning 😀 laughing 😆 sweat_smile 😅 joy 😂 rofl 🤣 wink 😉 blush 😊 innocent 😇
slightly_smiling_face 🙂 upside_down_face 🙃 heart_eyes 😍 star_struck 🤩 kissing_heart 😘
yum 😋 hugs 🤗 thinking 🤔 shushing_face 🤫 zipper_mouth_face 🤐 neutral_face 😐
expressionless 😑 roll_eyes 🙄 smirk 😏 grimacing 😬 relieved 😌 sleeping 😴 mask 😷
hot_face 🥵 cold_face 🥶 exploding_head 🤯 cowboy_hat_face 🤠 partying_face 🥳
sunglasses 😎 nerd_face 🤓 face_with_monocle 🧐 confused 😕 worried 😟 pleading_face 🥺
cry 😢 sob 😭 scream 😱 angry 😠 rage 😡 skull 💀 poop 💩 clown_face 🤡 ghost 👻 alien 👽
robot 🤖 see_no_evil 🙈 eyes 👀 brain 🧠 zzz 💤
+1 👍 thumbsup 👍 -1 👎 thumbsdown 👎 clap 👏 wave 👋 raised_hands 🙌 pray 🙏 ok_hand 👌
crossed_fingers 🤞 muscle 💪 point_right 👉 point_left 👈 point_up 👆 point_down 👇
handshake 🤝 writing_hand ✍️ fist 👊
heart ❤️ orange_heart 🧡 yellow_heart 💛 green_heart 💚 blue_heart 💙 purple_heart 💜
black_heart 🖤 broken_heart 💔 sparkling_heart 💖 two_hearts 💕
fire 🔥 sparkles ✨ star ⭐ star2 🌟 boom 💥 zap ⚡ 100 💯 tada 🎉 confetti_ball 🎊
balloon 🎈 gift 🎁 trophy 🏆 medal_sports 🏅 1st_place_medal 🥇 rocket 🚀 bulb 💡
warning ⚠️ cross_mark ❌ white_check_mark ✅ heavy_check_mark ✔️ question ❓ exclamation ❗
no_entry 🚫 construction 🚧 recycle ♻️ infinity ♾️ information_source ℹ️ sos 🆘 new 🆕
free 🆓 arrow_right ➡️ arrow_left ⬅️ arrow_up ⬆️ arrow_down ⬇️ checkered_flag 🏁
triangular_flag_on_post 🚩 white_flag 🏳️ copyright ©️ registered ®️
lock 🔒 unlock 🔓 key 🔑 link 🔗 pushpin 📌 paperclip 📎 memo 📝 pencil2 ✏️ book 📖
books 📚 bookmark 🔖 newspaper 📰 email 📧 envelope ✉️ inbox_tray 📥 outbox_tray 📤
package 📦 calendar 📆 date 📅 hourglass ⌛ hourglass_flowing_sand ⏳ alarm_clock ⏰
stopwatch ⏱️ chart_with_upwards_trend 📈 chart_with_downwards_trend 📉 bar_chart 📊
clipboard 📋 mag 🔍 gear ⚙️ hammer 🔨 wrench 🔧 toolbox 🧰 computer 💻 iphone 📱
keyboard ⌨️ desktop_computer 🖥️ printer 🖨️ camera 📷 movie_camera 🎥 tv 📺 radio 📻
headphones 🎧 microphone 🎤 musical_note 🎵 bell 🔔 speech_balloon 💬 thought_balloon 💭
globe_with_meridians 🌐 earth_africa 🌍 earth_americas 🌎 earth_asia 🌏
money_with_wings 💸 moneybag 💰 dollar 💵 euro 💶 pound 💷 yen 💴 credit_card 💳
shopping_cart 🛒 house 🏠 office 🏢 hospital 🏥 school 🏫 car 🚗 bus 🚌 train 🚆
airplane ✈️ bike 🚲 ship 🚢 world_map 🗺️ compass 🧭 mountain ⛰️ beach_umbrella 🏖️
camping 🏕️ sunny ☀️ cloud ☁️ umbrella ☂️ rainbow 🌈 snowflake ❄️ snowman ⛄ ocean 🌊
droplet 💧 crescent_moon 🌙 seedling 🌱 evergreen_tree 🌲 deciduous_tree 🌳 palm_tree 🌴
cactus 🌵 herb 🌿 four_leaf_clover 🍀 maple_leaf 🍁 fallen_leaf 🍂 sunflower 🌻 rose 🌹
tulip 🌷 cherry_blossom 🌸
dog 🐶 cat 🐱 mouse 🐭 rabbit 🐰 fox_face 🦊 bear 🐻 panda_face 🐼 koala 🐨 tiger 🐯
lion 🦁 cow 🐮 pig 🐷 frog 🐸 monkey 🐵 chicken 🐔 penguin 🐧 bird 🐦 owl 🦉 unicorn 🦄
bee 🐝 butterfly 🦋 snail 🐌 turtle 🐢 snake 🐍 octopus 🐙 fish 🐟 whale 🐳 dolphin 🐬
apple 🍎 banana 🍌 grapes 🍇 strawberry 🍓 lemon 🍋 avocado 🥑 carrot 🥕 corn 🌽
bread 🍞 cheese 🧀 egg 🥚 pizza 🍕 hamburger 🍔 fries 🍟 taco 🌮 sushi 🍣 ramen 🍜
cake 🍰 birthday 🎂 cookie 🍪 chocolate_bar 🍫 doughnut 🍩 coffee ☕ tea 🍵 beer 🍺
wine_glass 🍷 champagne 🍾
soccer ⚽ basketball 🏀 tennis 🎾 dart 🎯 video_game 🎮 jigsaw 🧩 art 🎨
performing_arts 🎭 guitar 🎸 runner 🏃
`;

/** Every emoji the menu knows, in the table's order. */
export const EMOJI: readonly Emoji[] = (() => {
  const words = TABLE.trim().split(/\s+/);
  const all: Emoji[] = [];
  for (let at = 0; at + 1 < words.length; at += 2) {
    const name = words[at];
    const emoji = words[at + 1];
    if (name !== undefined && emoji !== undefined) {
      all.push({ name, emoji });
    }
  }
  return all;
})();

/** How many emoji the menu shows at once. */
const SHOWN = 8;

/**
 * The emoji whose names hold `query`: the exact name first, then names that
 * start with it, then names with a word that starts with it, then the rest.
 */
export function findEmoji(query: string): Emoji[] {
  const wanted = query.toLowerCase();
  if (wanted === "") {
    return [];
  }
  const rank = ({ name }: Emoji) => {
    if (name === wanted) {
      return 0;
    }
    if (name.startsWith(wanted)) {
      return 1;
    }
    return name.includes(`_${wanted}`) ? 2 : 3;
  };
  const found = EMOJI.filter(({ name }) => name.includes(wanted));
  const ranked = found.map((emoji, order) => ({ emoji, order, rank: rank(emoji) }));
  ranked.sort((a, b) => a.rank - b.rank || a.order - b.order);
  return ranked.slice(0, SHOWN).map(({ emoji }) => emoji);
}
