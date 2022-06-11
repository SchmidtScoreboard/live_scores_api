use phf::phf_map;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Team {
    pub id: u32,
    pub location: &'static str,
    pub name: &'static str,
    pub display_name: &'static str,
    pub abbreviation: &'static str,
    pub primary_color: &'static str,
    pub secondary_color: &'static str,
}

impl Team {
    const fn new(
        id: u32,
        location: &'static str,
        name: &'static str,
        display_name: &'static str,
        abbreviation: &'static str,
        primary_color: &'static str,
        secondary_color: &'static str
    ) -> Team {
        Team {
            id,
            location,
            name,
            display_name,
            abbreviation,
            primary_color,
            secondary_color    
        }
    }

}


pub static BASEBALL_TEAMS: phf::Map<u64, Team> = phf_map! {
    108_u64 => Team::new(108, "Los Angeles", "Angels", "Angels", "LAA", "ba0021", "c4ced4"),
    109_u64 => Team::new(109, "Arizona", "D-backs", "D-backs", "ARI", "a71930", "e3d4ad"),
    110_u64 => Team::new(110, "Baltimore", "Orioles", "Orioles", "BAL", "df4601", "27251f"),
    111_u64 => Team::new(111, "Boston", "Red Sox", "Red Sox", "BOS", "c6011f", "ffffff"),
    112_u64 => Team::new(112, "Chicago", "Cubs", "Cubs", "CHC", "0e3386", "cc3433"),
    113_u64 => Team::new(113, "Cincinnati", "Reds", "Reds", "CIN", "c6011f", "000000"),
    114_u64 => Team::new(114, "Cleveland", "Indians", "Indians", "CLE", "e31937", "0c2340"),
    115_u64 => Team::new(115, "Colorado", "Rockies", "Rockies", "COL", "33006f", "c4ced4"),
    116_u64 => Team::new(116, "Detroit", "Tigers", "Tigers", "DET", "0c2340", "fa4616"),
    117_u64 => Team::new(117, "Houston", "Astros", "Astros", "HOU", "002d62", "f4911e"),
    118_u64 => Team::new(118, "Kansas City", "Royals", "Royals", "KC", "004687", "bd9b60"),
    119_u64 => Team::new(119, "Los Angeles", "Dodgers", "Dodgers", "LAD", "005a9c", "ef3e42"),
    120_u64 => Team::new(120, "Washington", "Nationals", "Nationals", "WSH", "ab0003", "14225a"),
    121_u64 => Team::new(121, "New York", "Mets", "Mets", "NYM", "002d72", "fc5910"),
    133_u64 => Team::new(133, "Oakland", "Athletics", "Athletics", "OAK", "003831", "efb21e"),
    134_u64 => Team::new(134, "Pittsburgh", "Pirates", "Pirates", "PIT", "fdb827", "27251f"),
    135_u64 => Team::new(135, "San Diego", "Padres", "Padres", "SD", "002d62", "a2aaad"),
    136_u64 => Team::new(136, "Seattle", "Mariners", "Mariners", "SEA", "005c5c", "c4ced4"),
    137_u64 => Team::new(137, "San Francisco", "Giants", "Giants", "SF", "27251f", "fd5a1e"),
    138_u64 => Team::new(138, "St. Louis", "Cardinals", "Cardinals", "STL", "c41e3a", "0c2340"),
    139_u64 => Team::new(139, "Tampa Bay", "Rays", "Rays", "TB", "d65a24", "ffffff"),
    140_u64 => Team::new(140, "Texas", "Rangers", "Rangers", "TEX", "003278", "c0111f"),
    141_u64 => Team::new(141, "Toronto", "Blue Jays", "Blue Jays", "TOR", "134a8e", "b1b3b3"),
    142_u64 => Team::new(142, "Minnesota", "Twins", "Twins", "MIN", "002b5c", "d31145"),
    143_u64 => Team::new(143, "Philadelphia", "Phillies", "Phillies", "PHI", "e81828", "002d72"),
    144_u64 => Team::new(144, "Atlanta", "Braves", "Braves", "ATL", "13274f", "ce1141"),
    145_u64 => Team::new(145, "Chicago", "White Sox", "White Sox", "CWS", "27251f", "c4ced4"),
    146_u64 => Team::new(146, "Miami", "Marlins", "Marlins", "MIA", "000000", "00a3e0"),
    147_u64 => Team::new(147, "New York", "Yankees", "Yankees", "NYY", "0c2340", "ffffff"),
    158_u64 => Team::new(158, "Milkwaukee", "Brewers", "Brewers", "MIL", "13294b", "b6922e"),
    159_u64 => Team::new(159, "NL", "NL All Stars", "NL All Stars", "NL", "ff0000", "ffffff"),
    160_u64 => Team::new(160, "AL", "AL All Stars", "AL All Stars", "AL", "0000ff", "ffffff"),
};

pub static HOCKEY_TEAMS: phf::Map<u64, Team> = phf_map! {
    1_u64 => Team::new(1, "New Jersey", "Devils", "Devils", "NJD", "c8102e", "000000"),
    2_u64 => Team::new(2, "New York", "Islanders", "Islanders", "NYI", "003087", "fc4c02"),
    3_u64 => Team::new(3, "New York", "Rangers", "Rangers", "NYR", "0033a0", "c8102e"),
    4_u64 => Team::new(4, "Philadelphia", "Flyers", "Flyers", "PHI", "fa4616", "000000"),
    5_u64 => Team::new(5, "Pittsburgh", "Penguins", "Penguins", "PIT", "ffb81c", "000000"),
    6_u64 => Team::new(6, "Boston", "Bruins", "Bruins", "BOS", "fcb514", "000000"),
    7_u64 => Team::new(7, "Buffalo", "Sabres", "Sabres", "BUF", "002654", "fcb514"),
    8_u64 => Team::new(8, "Montréal", "Canadiens", "Canadiens", "MTL", "a6192e", "001e62"),
    9_u64 => Team::new(9, "Ottawa", "Senators", "Senators", "OTT", "c8102e", "c69214"),
    10_u64 => Team::new(10, "Toronto", "Maple Leafs", "Leafs", "TOR", "00205b", "ffffff"),
    12_u64 => Team::new(12, "Carolina", "Hurricanes", "Canes", "CAR", "cc0000", "ffffff"),
    13_u64 => Team::new(13, "Florida", "Panthers", "Panthers", "FLA", "041e42", "b9975b"),
    14_u64 => Team::new(14, "Tampa Bay", "Lightning", "Lightning", "TBL", "00205b", "ffffff"),
    15_u64 => Team::new(15, "Washington", "Capitals", "Capitals", "WSH", "041e42", "c8102e"),
    16_u64 => Team::new(16, "Chicago", "Blackhawks", "B Hawks", "CHI", "ce1126", "000000"),
    17_u64 => Team::new(17, "Detroit", "Red Wings", "Red Wings", "DET", "c8102e", "ffffff"),
    18_u64 => Team::new(18, "Nashville", "Predators", "Predators", "NSH", "ffb81c", "041e42"),
    19_u64 => Team::new(19, "St. Louis", "Blues", "Blues", "STL", "002f87", "ffb81c"),
    20_u64 => Team::new(20, "Calgary", "Flames", "Flames", "CGY", "ce1126", "f3bc52"),
    21_u64 => Team::new(21, "Colorado", "Avalanche", "Avalanche", "COL", "236192", "d94574"),
    22_u64 => Team::new(22, "Edmonton", "Oilers", "Oilers", "EDM", "fc4c02", "041e42"),
    23_u64 => Team::new(23, "Vancouver", "Canucks", "Canucks", "VAN", "00843D", "ffffff"),
    24_u64 => Team::new(24, "Anaheim", "Ducks", "Ducks", "ANA", "b5985a", "ffffff"),
    25_u64 => Team::new(25, "Dallas", "Stars", "Stars", "DAL", "006341", "a2aaad"),
    26_u64 => Team::new(26, "Los Angeles", "Kings", "Kings", "LAK", "a2aaad", "000000"),
    28_u64 => Team::new(28, "San Jose", "Sharks", "Sharks", "SJS", "006272", "e57200"),
    29_u64 => Team::new(29, "Columbus", "Blue Jackets", "B Jackets", "CBJ", "041e42", "c8102e"),
    30_u64 => Team::new(30, "Minnesota", "Wild", "Wild", "MIN", "154734", "a6192e"),
    52_u64 => Team::new(52, "Winnipeg", "Jets", "Jets", "WPG", "041e42", "a2aaad"),
    53_u64 => Team::new(53, "Arizona", "Coyotes", "Coyotes", "ARI", "8c2633", "e2d6b5"),
    54_u64 => Team::new(54, "Las Vegas", "Golden Knights", "Knights", "VGK", "B4975A", "000000"),
    55_u64 => Team::new(55, "Seattle", "Kraken", "Kraken", "SEA", "001628", "99D9D9"),
    87_u64 => Team::new(87, "Atlantic", "Atlantic All Stars", "Atlantic", "ATL", "fa1b1b", "000000"),
    88_u64 => Team::new(88, "Metropolitan", "Metropolitan All Stars", "Metro", "MET", "fae71b", "000000"),
    89_u64 => Team::new(89, "Central", "Central All Stars", "Central", "CEN", "1411bd", "000000"),
    90_u64 => Team::new(90, "Pacific", "Pacific All Stars", "Pacific", "PAC", "11bd36", "000000"),
    7460_u64 => Team::new(7460, "Canada", "Canadian All Stars", "Canada", "CA", "d11717", "ffffff"),
    7461_u64 => Team::new(7461, "America", "American All Stars", "America", "USA", "3271a8", "ffffff"),
};
