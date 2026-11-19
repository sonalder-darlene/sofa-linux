// SPDX-License-Identifier: GPL-3.0-only
//
// Counted words in English ("1 bookmark", "3 bookmarks"). Both forms go through qsTr where they are written; real
// plural rules for other languages come with the translations.

.pragma library

function count(n, one, many) {
    return (n === 1 ? one : many).replace("%n", n);
}
