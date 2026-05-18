//! Annexe C table: 495-row lookup for third-placed teams in FWC26.
//! Auto-generated from FWC26_RULES.md §5. Do not edit manually.
//
//! Each entry: (qualifying_thirds_sorted, [winner_group, third_group, ...])
//! Winner groups (fixed): A, B, D, E, G, I, K, L

/// Each row: (sorted qualifying thirds [8 chars], mapping of (winner_group, third_group) pairs)
/// Winner groups in order: A, B, D, E, G, I, K, L
pub const ANNEXE_C: &[([u8; 8], [u8; 8])] = &[
    (
        [b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'F', b'H', b'G', b'L', b'K'],
    ), // option 1
    (
        [b'D', b'F', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'G', b'I', b'D', b'J', b'F', b'L', b'K'],
    ), // option 2
    (
        [b'D', b'E', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'D', b'H', b'G', b'L', b'K'],
    ), // option 3
    (
        [b'D', b'E', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'D', b'H', b'F', b'L', b'K'],
    ), // option 4
    (
        [b'D', b'E', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'G', b'I', b'D', b'J', b'F', b'L', b'K'],
    ), // option 5
    (
        [b'D', b'E', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'G', b'J', b'D', b'H', b'F', b'L', b'K'],
    ), // option 6
    (
        [b'D', b'E', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'I', b'D', b'H', b'F', b'L', b'K'],
    ), // option 7
    (
        [b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'G', b'J', b'D', b'H', b'F', b'L', b'I'],
    ), // option 8
    (
        [b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'G', b'J', b'D', b'H', b'F', b'I', b'K'],
    ), // option 9
    (
        [b'C', b'F', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'G', b'I', b'C', b'J', b'F', b'L', b'K'],
    ), // option 10
    (
        [b'C', b'E', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'C', b'H', b'G', b'L', b'K'],
    ), // option 11
    (
        [b'C', b'E', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'C', b'H', b'F', b'L', b'K'],
    ), // option 12
    (
        [b'C', b'E', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'G', b'I', b'C', b'J', b'F', b'L', b'K'],
    ), // option 13
    (
        [b'C', b'E', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'G', b'J', b'C', b'H', b'F', b'L', b'K'],
    ), // option 14
    (
        [b'C', b'E', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'I', b'C', b'H', b'F', b'L', b'K'],
    ), // option 15
    (
        [b'C', b'E', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'G', b'J', b'C', b'H', b'F', b'L', b'I'],
    ), // option 16
    (
        [b'C', b'E', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'G', b'J', b'C', b'H', b'F', b'I', b'K'],
    ), // option 17
    (
        [b'C', b'D', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'G', b'I', b'C', b'J', b'D', b'L', b'K'],
    ), // option 18
    (
        [b'C', b'D', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'C', b'J', b'I', b'D', b'H', b'F', b'L', b'K'],
    ), // option 19
    (
        [b'C', b'D', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'C', b'G', b'I', b'D', b'J', b'F', b'L', b'K'],
    ), // option 20
    (
        [b'C', b'D', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'C', b'G', b'J', b'D', b'H', b'F', b'L', b'K'],
    ), // option 21
    (
        [b'C', b'D', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'C', b'G', b'I', b'D', b'H', b'F', b'L', b'K'],
    ), // option 22
    (
        [b'C', b'D', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'C', b'G', b'J', b'D', b'H', b'F', b'L', b'I'],
    ), // option 23
    (
        [b'C', b'D', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'C', b'G', b'J', b'D', b'H', b'F', b'I', b'K'],
    ), // option 24
    (
        [b'C', b'D', b'E', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'C', b'H', b'D', b'L', b'K'],
    ), // option 25
    (
        [b'C', b'D', b'E', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'G', b'I', b'C', b'J', b'D', b'L', b'K'],
    ), // option 26
    (
        [b'C', b'D', b'E', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'G', b'J', b'C', b'H', b'D', b'L', b'K'],
    ), // option 27
    (
        [b'C', b'D', b'E', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'I', b'C', b'H', b'D', b'L', b'K'],
    ), // option 28
    (
        [b'C', b'D', b'E', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'G', b'J', b'C', b'H', b'D', b'L', b'I'],
    ), // option 29
    (
        [b'C', b'D', b'E', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'G', b'J', b'C', b'H', b'D', b'I', b'K'],
    ), // option 30
    (
        [b'C', b'D', b'E', b'F', b'I', b'J', b'K', b'L'],
        [b'C', b'J', b'E', b'D', b'I', b'F', b'L', b'K'],
    ), // option 31
    (
        [b'C', b'D', b'E', b'F', b'H', b'J', b'K', b'L'],
        [b'C', b'J', b'E', b'D', b'H', b'F', b'L', b'K'],
    ), // option 32
    (
        [b'C', b'D', b'E', b'F', b'H', b'I', b'K', b'L'],
        [b'C', b'E', b'I', b'D', b'H', b'F', b'L', b'K'],
    ), // option 33
    (
        [b'C', b'D', b'E', b'F', b'H', b'I', b'J', b'L'],
        [b'C', b'J', b'E', b'D', b'H', b'F', b'L', b'I'],
    ), // option 34
    (
        [b'C', b'D', b'E', b'F', b'H', b'I', b'J', b'K'],
        [b'C', b'J', b'E', b'D', b'H', b'F', b'I', b'K'],
    ), // option 35
    (
        [b'C', b'D', b'E', b'F', b'G', b'J', b'K', b'L'],
        [b'C', b'G', b'E', b'D', b'J', b'F', b'L', b'K'],
    ), // option 36
    (
        [b'C', b'D', b'E', b'F', b'G', b'I', b'K', b'L'],
        [b'C', b'G', b'E', b'D', b'I', b'F', b'L', b'K'],
    ), // option 37
    (
        [b'C', b'D', b'E', b'F', b'G', b'I', b'J', b'L'],
        [b'C', b'G', b'E', b'D', b'J', b'F', b'L', b'I'],
    ), // option 38
    (
        [b'C', b'D', b'E', b'F', b'G', b'I', b'J', b'K'],
        [b'C', b'G', b'E', b'D', b'J', b'F', b'I', b'K'],
    ), // option 39
    (
        [b'C', b'D', b'E', b'F', b'G', b'H', b'K', b'L'],
        [b'C', b'G', b'E', b'D', b'H', b'F', b'L', b'K'],
    ), // option 40
    (
        [b'C', b'D', b'E', b'F', b'G', b'H', b'J', b'L'],
        [b'C', b'G', b'J', b'D', b'H', b'F', b'L', b'E'],
    ), // option 41
    (
        [b'C', b'D', b'E', b'F', b'G', b'H', b'J', b'K'],
        [b'C', b'G', b'J', b'D', b'H', b'F', b'E', b'K'],
    ), // option 42
    (
        [b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'L'],
        [b'C', b'G', b'E', b'D', b'H', b'F', b'L', b'I'],
    ), // option 43
    (
        [b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'K'],
        [b'C', b'G', b'E', b'D', b'H', b'F', b'I', b'K'],
    ), // option 44
    (
        [b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J'],
        [b'C', b'G', b'J', b'D', b'H', b'F', b'E', b'I'],
    ), // option 45
    (
        [b'B', b'F', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'F', b'I', b'G', b'L', b'K'],
    ), // option 46
    (
        [b'B', b'E', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'B', b'H', b'G', b'L', b'K'],
    ), // option 47
    (
        [b'B', b'E', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'F', b'I', b'H', b'L', b'K'],
    ), // option 48
    (
        [b'B', b'E', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'F', b'I', b'G', b'L', b'K'],
    ), // option 49
    (
        [b'B', b'E', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'F', b'H', b'G', b'L', b'K'],
    ), // option 50
    (
        [b'B', b'E', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'F', b'I', b'H', b'L', b'K'],
    ), // option 51
    (
        [b'B', b'E', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'F', b'H', b'G', b'L', b'I'],
    ), // option 52
    (
        [b'B', b'E', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'F', b'H', b'G', b'I', b'K'],
    ), // option 53
    (
        [b'B', b'D', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'D', b'I', b'G', b'L', b'K'],
    ), // option 54
    (
        [b'B', b'D', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'D', b'I', b'F', b'L', b'K'],
    ), // option 55
    (
        [b'B', b'D', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'I', b'G', b'B', b'D', b'J', b'F', b'L', b'K'],
    ), // option 56
    (
        [b'B', b'D', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'H', b'G', b'B', b'D', b'J', b'F', b'L', b'K'],
    ), // option 57
    (
        [b'B', b'D', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'H', b'G', b'B', b'D', b'I', b'F', b'L', b'K'],
    ), // option 58
    (
        [b'B', b'D', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'H', b'G', b'B', b'D', b'J', b'F', b'L', b'I'],
    ), // option 59
    (
        [b'B', b'D', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'H', b'G', b'B', b'D', b'J', b'F', b'I', b'K'],
    ), // option 60
    (
        [b'B', b'D', b'E', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'D', b'I', b'H', b'L', b'K'],
    ), // option 61
    (
        [b'B', b'D', b'E', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'D', b'I', b'G', b'L', b'K'],
    ), // option 62
    (
        [b'B', b'D', b'E', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'D', b'H', b'G', b'L', b'K'],
    ), // option 63
    (
        [b'B', b'D', b'E', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'D', b'I', b'H', b'L', b'K'],
    ), // option 64
    (
        [b'B', b'D', b'E', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'D', b'H', b'G', b'L', b'I'],
    ), // option 65
    (
        [b'B', b'D', b'E', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'D', b'H', b'G', b'I', b'K'],
    ), // option 66
    (
        [b'B', b'D', b'E', b'F', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'D', b'I', b'F', b'L', b'K'],
    ), // option 67
    (
        [b'B', b'D', b'E', b'F', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'D', b'H', b'F', b'L', b'K'],
    ), // option 68
    (
        [b'B', b'D', b'E', b'F', b'H', b'I', b'K', b'L'],
        [b'E', b'I', b'B', b'D', b'H', b'F', b'L', b'K'],
    ), // option 69
    (
        [b'B', b'D', b'E', b'F', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'D', b'H', b'F', b'L', b'I'],
    ), // option 70
    (
        [b'B', b'D', b'E', b'F', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'D', b'H', b'F', b'I', b'K'],
    ), // option 71
    (
        [b'B', b'D', b'E', b'F', b'G', b'J', b'K', b'L'],
        [b'E', b'G', b'B', b'D', b'J', b'F', b'L', b'K'],
    ), // option 72
    (
        [b'B', b'D', b'E', b'F', b'G', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'D', b'I', b'F', b'L', b'K'],
    ), // option 73
    (
        [b'B', b'D', b'E', b'F', b'G', b'I', b'J', b'L'],
        [b'E', b'G', b'B', b'D', b'J', b'F', b'L', b'I'],
    ), // option 74
    (
        [b'B', b'D', b'E', b'F', b'G', b'I', b'J', b'K'],
        [b'E', b'G', b'B', b'D', b'J', b'F', b'I', b'K'],
    ), // option 75
    (
        [b'B', b'D', b'E', b'F', b'G', b'H', b'K', b'L'],
        [b'E', b'G', b'B', b'D', b'H', b'F', b'L', b'K'],
    ), // option 76
    (
        [b'B', b'D', b'E', b'F', b'G', b'H', b'J', b'L'],
        [b'H', b'G', b'B', b'D', b'J', b'F', b'L', b'E'],
    ), // option 77
    (
        [b'B', b'D', b'E', b'F', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'B', b'D', b'J', b'F', b'E', b'K'],
    ), // option 78
    (
        [b'B', b'D', b'E', b'F', b'G', b'H', b'I', b'L'],
        [b'E', b'G', b'B', b'D', b'H', b'F', b'L', b'I'],
    ), // option 79
    (
        [b'B', b'D', b'E', b'F', b'G', b'H', b'I', b'K'],
        [b'E', b'G', b'B', b'D', b'H', b'F', b'I', b'K'],
    ), // option 80
    (
        [b'B', b'D', b'E', b'F', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'B', b'D', b'J', b'F', b'E', b'I'],
    ), // option 81
    (
        [b'B', b'C', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'C', b'I', b'G', b'L', b'K'],
    ), // option 82
    (
        [b'B', b'C', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'C', b'I', b'F', b'L', b'K'],
    ), // option 83
    (
        [b'B', b'C', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'I', b'G', b'B', b'C', b'J', b'F', b'L', b'K'],
    ), // option 84
    (
        [b'B', b'C', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'H', b'G', b'B', b'C', b'J', b'F', b'L', b'K'],
    ), // option 85
    (
        [b'B', b'C', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'H', b'G', b'B', b'C', b'I', b'F', b'L', b'K'],
    ), // option 86
    (
        [b'B', b'C', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'H', b'G', b'B', b'C', b'J', b'F', b'L', b'I'],
    ), // option 87
    (
        [b'B', b'C', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'H', b'G', b'B', b'C', b'J', b'F', b'I', b'K'],
    ), // option 88
    (
        [b'B', b'C', b'E', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'I', b'H', b'L', b'K'],
    ), // option 89
    (
        [b'B', b'C', b'E', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'I', b'G', b'L', b'K'],
    ), // option 90
    (
        [b'B', b'C', b'E', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'H', b'G', b'L', b'K'],
    ), // option 91
    (
        [b'B', b'C', b'E', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'I', b'H', b'L', b'K'],
    ), // option 92
    (
        [b'B', b'C', b'E', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'C', b'H', b'G', b'L', b'I'],
    ), // option 93
    (
        [b'B', b'C', b'E', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'C', b'H', b'G', b'I', b'K'],
    ), // option 94
    (
        [b'B', b'C', b'E', b'F', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'I', b'F', b'L', b'K'],
    ), // option 95
    (
        [b'B', b'C', b'E', b'F', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'H', b'F', b'L', b'K'],
    ), // option 96
    (
        [b'B', b'C', b'E', b'F', b'H', b'I', b'K', b'L'],
        [b'E', b'I', b'B', b'C', b'H', b'F', b'L', b'K'],
    ), // option 97
    (
        [b'B', b'C', b'E', b'F', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'C', b'H', b'F', b'L', b'I'],
    ), // option 98
    (
        [b'B', b'C', b'E', b'F', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'C', b'H', b'F', b'I', b'K'],
    ), // option 99
    (
        [b'B', b'C', b'E', b'F', b'G', b'J', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'J', b'F', b'L', b'K'],
    ), // option 100
    (
        [b'B', b'C', b'E', b'F', b'G', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'I', b'F', b'L', b'K'],
    ), // option 101
    (
        [b'B', b'C', b'E', b'F', b'G', b'I', b'J', b'L'],
        [b'E', b'G', b'B', b'C', b'J', b'F', b'L', b'I'],
    ), // option 102
    (
        [b'B', b'C', b'E', b'F', b'G', b'I', b'J', b'K'],
        [b'E', b'G', b'B', b'C', b'J', b'F', b'I', b'K'],
    ), // option 103
    (
        [b'B', b'C', b'E', b'F', b'G', b'H', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'H', b'F', b'L', b'K'],
    ), // option 104
    (
        [b'B', b'C', b'E', b'F', b'G', b'H', b'J', b'L'],
        [b'H', b'G', b'B', b'C', b'J', b'F', b'L', b'E'],
    ), // option 105
    (
        [b'B', b'C', b'E', b'F', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'B', b'C', b'J', b'F', b'E', b'K'],
    ), // option 106
    (
        [b'B', b'C', b'E', b'F', b'G', b'H', b'I', b'L'],
        [b'E', b'G', b'B', b'C', b'H', b'F', b'L', b'I'],
    ), // option 107
    (
        [b'B', b'C', b'E', b'F', b'G', b'H', b'I', b'K'],
        [b'E', b'G', b'B', b'C', b'H', b'F', b'I', b'K'],
    ), // option 108
    (
        [b'B', b'C', b'E', b'F', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'B', b'C', b'J', b'F', b'E', b'I'],
    ), // option 109
    (
        [b'B', b'C', b'D', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'C', b'I', b'D', b'L', b'K'],
    ), // option 110
    (
        [b'B', b'C', b'D', b'G', b'I', b'J', b'K', b'L'],
        [b'I', b'G', b'B', b'C', b'J', b'D', b'L', b'K'],
    ), // option 111
    (
        [b'B', b'C', b'D', b'G', b'H', b'J', b'K', b'L'],
        [b'H', b'G', b'B', b'C', b'J', b'D', b'L', b'K'],
    ), // option 112
    (
        [b'B', b'C', b'D', b'G', b'H', b'I', b'K', b'L'],
        [b'H', b'G', b'B', b'C', b'I', b'D', b'L', b'K'],
    ), // option 113
    (
        [b'B', b'C', b'D', b'G', b'H', b'I', b'J', b'L'],
        [b'H', b'G', b'B', b'C', b'J', b'D', b'L', b'I'],
    ), // option 114
    (
        [b'B', b'C', b'D', b'G', b'H', b'I', b'J', b'K'],
        [b'H', b'G', b'B', b'C', b'J', b'D', b'I', b'K'],
    ), // option 115
    (
        [b'B', b'C', b'D', b'F', b'I', b'J', b'K', b'L'],
        [b'C', b'J', b'B', b'D', b'I', b'F', b'L', b'K'],
    ), // option 116
    (
        [b'B', b'C', b'D', b'F', b'H', b'J', b'K', b'L'],
        [b'C', b'J', b'B', b'D', b'H', b'F', b'L', b'K'],
    ), // option 117
    (
        [b'B', b'C', b'D', b'F', b'H', b'I', b'K', b'L'],
        [b'C', b'I', b'B', b'D', b'H', b'F', b'L', b'K'],
    ), // option 118
    (
        [b'B', b'C', b'D', b'F', b'H', b'I', b'J', b'L'],
        [b'C', b'J', b'B', b'D', b'H', b'F', b'L', b'I'],
    ), // option 119
    (
        [b'B', b'C', b'D', b'F', b'H', b'I', b'J', b'K'],
        [b'C', b'J', b'B', b'D', b'H', b'F', b'I', b'K'],
    ), // option 120
    (
        [b'B', b'C', b'D', b'F', b'G', b'J', b'K', b'L'],
        [b'C', b'G', b'B', b'D', b'J', b'F', b'L', b'K'],
    ), // option 121
    (
        [b'B', b'C', b'D', b'F', b'G', b'I', b'K', b'L'],
        [b'C', b'G', b'B', b'D', b'I', b'F', b'L', b'K'],
    ), // option 122
    (
        [b'B', b'C', b'D', b'F', b'G', b'I', b'J', b'L'],
        [b'C', b'G', b'B', b'D', b'J', b'F', b'L', b'I'],
    ), // option 123
    (
        [b'B', b'C', b'D', b'F', b'G', b'I', b'J', b'K'],
        [b'C', b'G', b'B', b'D', b'J', b'F', b'I', b'K'],
    ), // option 124
    (
        [b'B', b'C', b'D', b'F', b'G', b'H', b'K', b'L'],
        [b'C', b'G', b'B', b'D', b'H', b'F', b'L', b'K'],
    ), // option 125
    (
        [b'B', b'C', b'D', b'F', b'G', b'H', b'J', b'L'],
        [b'C', b'G', b'B', b'D', b'H', b'F', b'L', b'J'],
    ), // option 126
    (
        [b'B', b'C', b'D', b'F', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'B', b'C', b'J', b'F', b'D', b'K'],
    ), // option 127
    (
        [b'B', b'C', b'D', b'F', b'G', b'H', b'I', b'L'],
        [b'C', b'G', b'B', b'D', b'H', b'F', b'L', b'I'],
    ), // option 128
    (
        [b'B', b'C', b'D', b'F', b'G', b'H', b'I', b'K'],
        [b'C', b'G', b'B', b'D', b'H', b'F', b'I', b'K'],
    ), // option 129
    (
        [b'B', b'C', b'D', b'F', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'B', b'C', b'J', b'F', b'D', b'I'],
    ), // option 130
    (
        [b'B', b'C', b'D', b'E', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'I', b'D', b'L', b'K'],
    ), // option 131
    (
        [b'B', b'C', b'D', b'E', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'H', b'D', b'L', b'K'],
    ), // option 132
    (
        [b'B', b'C', b'D', b'E', b'H', b'I', b'K', b'L'],
        [b'E', b'I', b'B', b'C', b'H', b'D', b'L', b'K'],
    ), // option 133
    (
        [b'B', b'C', b'D', b'E', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'C', b'H', b'D', b'L', b'I'],
    ), // option 134
    (
        [b'B', b'C', b'D', b'E', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'C', b'H', b'D', b'I', b'K'],
    ), // option 135
    (
        [b'B', b'C', b'D', b'E', b'G', b'J', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'J', b'D', b'L', b'K'],
    ), // option 136
    (
        [b'B', b'C', b'D', b'E', b'G', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'I', b'D', b'L', b'K'],
    ), // option 137
    (
        [b'B', b'C', b'D', b'E', b'G', b'I', b'J', b'L'],
        [b'E', b'G', b'B', b'C', b'J', b'D', b'L', b'I'],
    ), // option 138
    (
        [b'B', b'C', b'D', b'E', b'G', b'I', b'J', b'K'],
        [b'E', b'G', b'B', b'C', b'J', b'D', b'I', b'K'],
    ), // option 139
    (
        [b'B', b'C', b'D', b'E', b'G', b'H', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'H', b'D', b'L', b'K'],
    ), // option 140
    (
        [b'B', b'C', b'D', b'E', b'G', b'H', b'J', b'L'],
        [b'H', b'G', b'B', b'C', b'J', b'D', b'L', b'E'],
    ), // option 141
    (
        [b'B', b'C', b'D', b'E', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'B', b'C', b'J', b'D', b'E', b'K'],
    ), // option 142
    (
        [b'B', b'C', b'D', b'E', b'G', b'H', b'I', b'L'],
        [b'E', b'G', b'B', b'C', b'H', b'D', b'L', b'I'],
    ), // option 143
    (
        [b'B', b'C', b'D', b'E', b'G', b'H', b'I', b'K'],
        [b'E', b'G', b'B', b'C', b'H', b'D', b'I', b'K'],
    ), // option 144
    (
        [b'B', b'C', b'D', b'E', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'B', b'C', b'J', b'D', b'E', b'I'],
    ), // option 145
    (
        [b'B', b'C', b'D', b'E', b'F', b'J', b'K', b'L'],
        [b'C', b'J', b'B', b'D', b'E', b'F', b'L', b'K'],
    ), // option 146
    (
        [b'B', b'C', b'D', b'E', b'F', b'I', b'K', b'L'],
        [b'C', b'E', b'B', b'D', b'I', b'F', b'L', b'K'],
    ), // option 147
    (
        [b'B', b'C', b'D', b'E', b'F', b'I', b'J', b'L'],
        [b'C', b'J', b'B', b'D', b'E', b'F', b'L', b'I'],
    ), // option 148
    (
        [b'B', b'C', b'D', b'E', b'F', b'I', b'J', b'K'],
        [b'C', b'J', b'B', b'D', b'E', b'F', b'I', b'K'],
    ), // option 149
    (
        [b'B', b'C', b'D', b'E', b'F', b'H', b'K', b'L'],
        [b'C', b'E', b'B', b'D', b'H', b'F', b'L', b'K'],
    ), // option 150
    (
        [b'B', b'C', b'D', b'E', b'F', b'H', b'J', b'L'],
        [b'C', b'J', b'B', b'D', b'H', b'F', b'L', b'E'],
    ), // option 151
    (
        [b'B', b'C', b'D', b'E', b'F', b'H', b'J', b'K'],
        [b'C', b'J', b'B', b'D', b'H', b'F', b'E', b'K'],
    ), // option 152
    (
        [b'B', b'C', b'D', b'E', b'F', b'H', b'I', b'L'],
        [b'C', b'E', b'B', b'D', b'H', b'F', b'L', b'I'],
    ), // option 153
    (
        [b'B', b'C', b'D', b'E', b'F', b'H', b'I', b'K'],
        [b'C', b'E', b'B', b'D', b'H', b'F', b'I', b'K'],
    ), // option 154
    (
        [b'B', b'C', b'D', b'E', b'F', b'H', b'I', b'J'],
        [b'C', b'J', b'B', b'D', b'H', b'F', b'E', b'I'],
    ), // option 155
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'K', b'L'],
        [b'C', b'G', b'B', b'D', b'E', b'F', b'L', b'K'],
    ), // option 156
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'J', b'L'],
        [b'C', b'G', b'B', b'D', b'J', b'F', b'L', b'E'],
    ), // option 157
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'J', b'K'],
        [b'C', b'G', b'B', b'D', b'J', b'F', b'E', b'K'],
    ), // option 158
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'I', b'L'],
        [b'C', b'G', b'B', b'D', b'E', b'F', b'L', b'I'],
    ), // option 159
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'I', b'K'],
        [b'C', b'G', b'B', b'D', b'E', b'F', b'I', b'K'],
    ), // option 160
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'I', b'J'],
        [b'C', b'G', b'B', b'D', b'J', b'F', b'E', b'I'],
    ), // option 161
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'L'],
        [b'C', b'G', b'B', b'D', b'H', b'F', b'L', b'E'],
    ), // option 162
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'K'],
        [b'C', b'G', b'B', b'D', b'H', b'F', b'E', b'K'],
    ), // option 163
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'J'],
        [b'H', b'G', b'B', b'C', b'J', b'F', b'D', b'E'],
    ), // option 164
    (
        [b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I'],
        [b'C', b'G', b'B', b'D', b'H', b'F', b'E', b'I'],
    ), // option 165
    (
        [b'A', b'F', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'I', b'F', b'A', b'G', b'L', b'K'],
    ), // option 166
    (
        [b'A', b'E', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'A', b'H', b'G', b'L', b'K'],
    ), // option 167
    (
        [b'A', b'E', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'F', b'A', b'H', b'L', b'K'],
    ), // option 168
    (
        [b'A', b'E', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'F', b'A', b'G', b'L', b'K'],
    ), // option 169
    (
        [b'A', b'E', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'G', b'J', b'F', b'A', b'H', b'L', b'K'],
    ), // option 170
    (
        [b'A', b'E', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'I', b'F', b'A', b'H', b'L', b'K'],
    ), // option 171
    (
        [b'A', b'E', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'G', b'J', b'F', b'A', b'H', b'L', b'I'],
    ), // option 172
    (
        [b'A', b'E', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'G', b'J', b'F', b'A', b'H', b'I', b'K'],
    ), // option 173
    (
        [b'A', b'D', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'I', b'D', b'A', b'G', b'L', b'K'],
    ), // option 174
    (
        [b'A', b'D', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'I', b'D', b'A', b'F', b'L', b'K'],
    ), // option 175
    (
        [b'A', b'D', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'I', b'G', b'J', b'D', b'A', b'F', b'L', b'K'],
    ), // option 176
    (
        [b'A', b'D', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'H', b'G', b'J', b'D', b'A', b'F', b'L', b'K'],
    ), // option 177
    (
        [b'A', b'D', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'H', b'G', b'I', b'D', b'A', b'F', b'L', b'K'],
    ), // option 178
    (
        [b'A', b'D', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'H', b'G', b'J', b'D', b'A', b'F', b'L', b'I'],
    ), // option 179
    (
        [b'A', b'D', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'H', b'G', b'J', b'D', b'A', b'F', b'I', b'K'],
    ), // option 180
    (
        [b'A', b'D', b'E', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'D', b'A', b'H', b'L', b'K'],
    ), // option 181
    (
        [b'A', b'D', b'E', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'D', b'A', b'G', b'L', b'K'],
    ), // option 182
    (
        [b'A', b'D', b'E', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'G', b'J', b'D', b'A', b'H', b'L', b'K'],
    ), // option 183
    (
        [b'A', b'D', b'E', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'I', b'D', b'A', b'H', b'L', b'K'],
    ), // option 184
    (
        [b'A', b'D', b'E', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'G', b'J', b'D', b'A', b'H', b'L', b'I'],
    ), // option 185
    (
        [b'A', b'D', b'E', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'G', b'J', b'D', b'A', b'H', b'I', b'K'],
    ), // option 186
    (
        [b'A', b'D', b'E', b'F', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'D', b'A', b'F', b'L', b'K'],
    ), // option 187
    (
        [b'A', b'D', b'E', b'F', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'E', b'D', b'A', b'F', b'L', b'K'],
    ), // option 188
    (
        [b'A', b'D', b'E', b'F', b'H', b'I', b'K', b'L'],
        [b'H', b'E', b'I', b'D', b'A', b'F', b'L', b'K'],
    ), // option 189
    (
        [b'A', b'D', b'E', b'F', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'E', b'D', b'A', b'F', b'L', b'I'],
    ), // option 190
    (
        [b'A', b'D', b'E', b'F', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'E', b'D', b'A', b'F', b'I', b'K'],
    ), // option 191
    (
        [b'A', b'D', b'E', b'F', b'G', b'J', b'K', b'L'],
        [b'E', b'G', b'J', b'D', b'A', b'F', b'L', b'K'],
    ), // option 192
    (
        [b'A', b'D', b'E', b'F', b'G', b'I', b'K', b'L'],
        [b'E', b'G', b'I', b'D', b'A', b'F', b'L', b'K'],
    ), // option 193
    (
        [b'A', b'D', b'E', b'F', b'G', b'I', b'J', b'L'],
        [b'E', b'G', b'J', b'D', b'A', b'F', b'L', b'I'],
    ), // option 194
    (
        [b'A', b'D', b'E', b'F', b'G', b'I', b'J', b'K'],
        [b'E', b'G', b'J', b'D', b'A', b'F', b'I', b'K'],
    ), // option 195
    (
        [b'A', b'D', b'E', b'F', b'G', b'H', b'K', b'L'],
        [b'H', b'G', b'E', b'D', b'A', b'F', b'L', b'K'],
    ), // option 196
    (
        [b'A', b'D', b'E', b'F', b'G', b'H', b'J', b'L'],
        [b'H', b'G', b'J', b'D', b'A', b'F', b'L', b'E'],
    ), // option 197
    (
        [b'A', b'D', b'E', b'F', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'J', b'D', b'A', b'F', b'E', b'K'],
    ), // option 198
    (
        [b'A', b'D', b'E', b'F', b'G', b'H', b'I', b'L'],
        [b'H', b'G', b'E', b'D', b'A', b'F', b'L', b'I'],
    ), // option 199
    (
        [b'A', b'D', b'E', b'F', b'G', b'H', b'I', b'K'],
        [b'H', b'G', b'E', b'D', b'A', b'F', b'I', b'K'],
    ), // option 200
    (
        [b'A', b'D', b'E', b'F', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'J', b'D', b'A', b'F', b'E', b'I'],
    ), // option 201
    (
        [b'A', b'C', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'I', b'C', b'A', b'G', b'L', b'K'],
    ), // option 202
    (
        [b'A', b'C', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'I', b'C', b'A', b'F', b'L', b'K'],
    ), // option 203
    (
        [b'A', b'C', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'I', b'G', b'J', b'C', b'A', b'F', b'L', b'K'],
    ), // option 204
    (
        [b'A', b'C', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'H', b'G', b'J', b'C', b'A', b'F', b'L', b'K'],
    ), // option 205
    (
        [b'A', b'C', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'H', b'G', b'I', b'C', b'A', b'F', b'L', b'K'],
    ), // option 206
    (
        [b'A', b'C', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'H', b'G', b'J', b'C', b'A', b'F', b'L', b'I'],
    ), // option 207
    (
        [b'A', b'C', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'H', b'G', b'J', b'C', b'A', b'F', b'I', b'K'],
    ), // option 208
    (
        [b'A', b'C', b'E', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'C', b'A', b'H', b'L', b'K'],
    ), // option 209
    (
        [b'A', b'C', b'E', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'C', b'A', b'G', b'L', b'K'],
    ), // option 210
    (
        [b'A', b'C', b'E', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'G', b'J', b'C', b'A', b'H', b'L', b'K'],
    ), // option 211
    (
        [b'A', b'C', b'E', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'I', b'C', b'A', b'H', b'L', b'K'],
    ), // option 212
    (
        [b'A', b'C', b'E', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'G', b'J', b'C', b'A', b'H', b'L', b'I'],
    ), // option 213
    (
        [b'A', b'C', b'E', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'G', b'J', b'C', b'A', b'H', b'I', b'K'],
    ), // option 214
    (
        [b'A', b'C', b'E', b'F', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'C', b'A', b'F', b'L', b'K'],
    ), // option 215
    (
        [b'A', b'C', b'E', b'F', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'E', b'C', b'A', b'F', b'L', b'K'],
    ), // option 216
    (
        [b'A', b'C', b'E', b'F', b'H', b'I', b'K', b'L'],
        [b'H', b'E', b'I', b'C', b'A', b'F', b'L', b'K'],
    ), // option 217
    (
        [b'A', b'C', b'E', b'F', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'E', b'C', b'A', b'F', b'L', b'I'],
    ), // option 218
    (
        [b'A', b'C', b'E', b'F', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'E', b'C', b'A', b'F', b'I', b'K'],
    ), // option 219
    (
        [b'A', b'C', b'E', b'F', b'G', b'J', b'K', b'L'],
        [b'E', b'G', b'J', b'C', b'A', b'F', b'L', b'K'],
    ), // option 220
    (
        [b'A', b'C', b'E', b'F', b'G', b'I', b'K', b'L'],
        [b'E', b'G', b'I', b'C', b'A', b'F', b'L', b'K'],
    ), // option 221
    (
        [b'A', b'C', b'E', b'F', b'G', b'I', b'J', b'L'],
        [b'E', b'G', b'J', b'C', b'A', b'F', b'L', b'I'],
    ), // option 222
    (
        [b'A', b'C', b'E', b'F', b'G', b'I', b'J', b'K'],
        [b'E', b'G', b'J', b'C', b'A', b'F', b'I', b'K'],
    ), // option 223
    (
        [b'A', b'C', b'E', b'F', b'G', b'H', b'K', b'L'],
        [b'H', b'G', b'E', b'C', b'A', b'F', b'L', b'K'],
    ), // option 224
    (
        [b'A', b'C', b'E', b'F', b'G', b'H', b'J', b'L'],
        [b'H', b'G', b'J', b'C', b'A', b'F', b'L', b'E'],
    ), // option 225
    (
        [b'A', b'C', b'E', b'F', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'J', b'C', b'A', b'F', b'E', b'K'],
    ), // option 226
    (
        [b'A', b'C', b'E', b'F', b'G', b'H', b'I', b'L'],
        [b'H', b'G', b'E', b'C', b'A', b'F', b'L', b'I'],
    ), // option 227
    (
        [b'A', b'C', b'E', b'F', b'G', b'H', b'I', b'K'],
        [b'H', b'G', b'E', b'C', b'A', b'F', b'I', b'K'],
    ), // option 228
    (
        [b'A', b'C', b'E', b'F', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'J', b'C', b'A', b'F', b'E', b'I'],
    ), // option 229
    (
        [b'A', b'C', b'D', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'I', b'C', b'A', b'D', b'L', b'K'],
    ), // option 230
    (
        [b'A', b'C', b'D', b'G', b'I', b'J', b'K', b'L'],
        [b'I', b'G', b'J', b'C', b'A', b'D', b'L', b'K'],
    ), // option 231
    (
        [b'A', b'C', b'D', b'G', b'H', b'J', b'K', b'L'],
        [b'H', b'G', b'J', b'C', b'A', b'D', b'L', b'K'],
    ), // option 232
    (
        [b'A', b'C', b'D', b'G', b'H', b'I', b'K', b'L'],
        [b'H', b'G', b'I', b'C', b'A', b'D', b'L', b'K'],
    ), // option 233
    (
        [b'A', b'C', b'D', b'G', b'H', b'I', b'J', b'L'],
        [b'H', b'G', b'J', b'C', b'A', b'D', b'L', b'I'],
    ), // option 234
    (
        [b'A', b'C', b'D', b'G', b'H', b'I', b'J', b'K'],
        [b'H', b'G', b'J', b'C', b'A', b'D', b'I', b'K'],
    ), // option 235
    (
        [b'A', b'C', b'D', b'F', b'I', b'J', b'K', b'L'],
        [b'C', b'J', b'I', b'D', b'A', b'F', b'L', b'K'],
    ), // option 236
    (
        [b'A', b'C', b'D', b'F', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'F', b'C', b'A', b'D', b'L', b'K'],
    ), // option 237
    (
        [b'A', b'C', b'D', b'F', b'H', b'I', b'K', b'L'],
        [b'H', b'F', b'I', b'C', b'A', b'D', b'L', b'K'],
    ), // option 238
    (
        [b'A', b'C', b'D', b'F', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'F', b'C', b'A', b'D', b'L', b'I'],
    ), // option 239
    (
        [b'A', b'C', b'D', b'F', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'F', b'C', b'A', b'D', b'I', b'K'],
    ), // option 240
    (
        [b'A', b'C', b'D', b'F', b'G', b'J', b'K', b'L'],
        [b'C', b'G', b'J', b'D', b'A', b'F', b'L', b'K'],
    ), // option 241
    (
        [b'A', b'C', b'D', b'F', b'G', b'I', b'K', b'L'],
        [b'C', b'G', b'I', b'D', b'A', b'F', b'L', b'K'],
    ), // option 242
    (
        [b'A', b'C', b'D', b'F', b'G', b'I', b'J', b'L'],
        [b'C', b'G', b'J', b'D', b'A', b'F', b'L', b'I'],
    ), // option 243
    (
        [b'A', b'C', b'D', b'F', b'G', b'I', b'J', b'K'],
        [b'C', b'G', b'J', b'D', b'A', b'F', b'I', b'K'],
    ), // option 244
    (
        [b'A', b'C', b'D', b'F', b'G', b'H', b'K', b'L'],
        [b'H', b'G', b'F', b'C', b'A', b'D', b'L', b'K'],
    ), // option 245
    (
        [b'A', b'C', b'D', b'F', b'G', b'H', b'J', b'L'],
        [b'C', b'G', b'J', b'D', b'A', b'F', b'L', b'H'],
    ), // option 246
    (
        [b'A', b'C', b'D', b'F', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'J', b'C', b'A', b'F', b'D', b'K'],
    ), // option 247
    (
        [b'A', b'C', b'D', b'F', b'G', b'H', b'I', b'L'],
        [b'H', b'G', b'F', b'C', b'A', b'D', b'L', b'I'],
    ), // option 248
    (
        [b'A', b'C', b'D', b'F', b'G', b'H', b'I', b'K'],
        [b'H', b'G', b'F', b'C', b'A', b'D', b'I', b'K'],
    ), // option 249
    (
        [b'A', b'C', b'D', b'F', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'J', b'C', b'A', b'F', b'D', b'I'],
    ), // option 250
    (
        [b'A', b'C', b'D', b'E', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'I', b'C', b'A', b'D', b'L', b'K'],
    ), // option 251
    (
        [b'A', b'C', b'D', b'E', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'E', b'C', b'A', b'D', b'L', b'K'],
    ), // option 252
    (
        [b'A', b'C', b'D', b'E', b'H', b'I', b'K', b'L'],
        [b'H', b'E', b'I', b'C', b'A', b'D', b'L', b'K'],
    ), // option 253
    (
        [b'A', b'C', b'D', b'E', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'E', b'C', b'A', b'D', b'L', b'I'],
    ), // option 254
    (
        [b'A', b'C', b'D', b'E', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'E', b'C', b'A', b'D', b'I', b'K'],
    ), // option 255
    (
        [b'A', b'C', b'D', b'E', b'G', b'J', b'K', b'L'],
        [b'E', b'G', b'J', b'C', b'A', b'D', b'L', b'K'],
    ), // option 256
    (
        [b'A', b'C', b'D', b'E', b'G', b'I', b'K', b'L'],
        [b'E', b'G', b'I', b'C', b'A', b'D', b'L', b'K'],
    ), // option 257
    (
        [b'A', b'C', b'D', b'E', b'G', b'I', b'J', b'L'],
        [b'E', b'G', b'J', b'C', b'A', b'D', b'L', b'I'],
    ), // option 258
    (
        [b'A', b'C', b'D', b'E', b'G', b'I', b'J', b'K'],
        [b'E', b'G', b'J', b'C', b'A', b'D', b'I', b'K'],
    ), // option 259
    (
        [b'A', b'C', b'D', b'E', b'G', b'H', b'K', b'L'],
        [b'H', b'G', b'E', b'C', b'A', b'D', b'L', b'K'],
    ), // option 260
    (
        [b'A', b'C', b'D', b'E', b'G', b'H', b'J', b'L'],
        [b'H', b'G', b'J', b'C', b'A', b'D', b'L', b'E'],
    ), // option 261
    (
        [b'A', b'C', b'D', b'E', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'J', b'C', b'A', b'D', b'E', b'K'],
    ), // option 262
    (
        [b'A', b'C', b'D', b'E', b'G', b'H', b'I', b'L'],
        [b'H', b'G', b'E', b'C', b'A', b'D', b'L', b'I'],
    ), // option 263
    (
        [b'A', b'C', b'D', b'E', b'G', b'H', b'I', b'K'],
        [b'H', b'G', b'E', b'C', b'A', b'D', b'I', b'K'],
    ), // option 264
    (
        [b'A', b'C', b'D', b'E', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'J', b'C', b'A', b'D', b'E', b'I'],
    ), // option 265
    (
        [b'A', b'C', b'D', b'E', b'F', b'J', b'K', b'L'],
        [b'C', b'J', b'E', b'D', b'A', b'F', b'L', b'K'],
    ), // option 266
    (
        [b'A', b'C', b'D', b'E', b'F', b'I', b'K', b'L'],
        [b'C', b'E', b'I', b'D', b'A', b'F', b'L', b'K'],
    ), // option 267
    (
        [b'A', b'C', b'D', b'E', b'F', b'I', b'J', b'L'],
        [b'C', b'J', b'E', b'D', b'A', b'F', b'L', b'I'],
    ), // option 268
    (
        [b'A', b'C', b'D', b'E', b'F', b'I', b'J', b'K'],
        [b'C', b'J', b'E', b'D', b'A', b'F', b'I', b'K'],
    ), // option 269
    (
        [b'A', b'C', b'D', b'E', b'F', b'H', b'K', b'L'],
        [b'H', b'E', b'F', b'C', b'A', b'D', b'L', b'K'],
    ), // option 270
    (
        [b'A', b'C', b'D', b'E', b'F', b'H', b'J', b'L'],
        [b'H', b'J', b'F', b'C', b'A', b'D', b'L', b'E'],
    ), // option 271
    (
        [b'A', b'C', b'D', b'E', b'F', b'H', b'J', b'K'],
        [b'H', b'J', b'E', b'C', b'A', b'F', b'D', b'K'],
    ), // option 272
    (
        [b'A', b'C', b'D', b'E', b'F', b'H', b'I', b'L'],
        [b'H', b'E', b'F', b'C', b'A', b'D', b'L', b'I'],
    ), // option 273
    (
        [b'A', b'C', b'D', b'E', b'F', b'H', b'I', b'K'],
        [b'H', b'E', b'F', b'C', b'A', b'D', b'I', b'K'],
    ), // option 274
    (
        [b'A', b'C', b'D', b'E', b'F', b'H', b'I', b'J'],
        [b'H', b'J', b'E', b'C', b'A', b'F', b'D', b'I'],
    ), // option 275
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'K', b'L'],
        [b'C', b'G', b'E', b'D', b'A', b'F', b'L', b'K'],
    ), // option 276
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'J', b'L'],
        [b'C', b'G', b'J', b'D', b'A', b'F', b'L', b'E'],
    ), // option 277
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'J', b'K'],
        [b'C', b'G', b'J', b'D', b'A', b'F', b'E', b'K'],
    ), // option 278
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'I', b'L'],
        [b'C', b'G', b'E', b'D', b'A', b'F', b'L', b'I'],
    ), // option 279
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'I', b'K'],
        [b'C', b'G', b'E', b'D', b'A', b'F', b'I', b'K'],
    ), // option 280
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'I', b'J'],
        [b'C', b'G', b'J', b'D', b'A', b'F', b'E', b'I'],
    ), // option 281
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'H', b'L'],
        [b'H', b'G', b'F', b'C', b'A', b'D', b'L', b'E'],
    ), // option 282
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'H', b'K'],
        [b'H', b'G', b'E', b'C', b'A', b'F', b'D', b'K'],
    ), // option 283
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'H', b'J'],
        [b'H', b'G', b'J', b'C', b'A', b'F', b'D', b'E'],
    ), // option 284
    (
        [b'A', b'C', b'D', b'E', b'F', b'G', b'H', b'I'],
        [b'H', b'G', b'E', b'C', b'A', b'F', b'D', b'I'],
    ), // option 285
    (
        [b'A', b'B', b'G', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'A', b'I', b'G', b'L', b'K'],
    ), // option 286
    (
        [b'A', b'B', b'F', b'H', b'I', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'A', b'I', b'F', b'L', b'K'],
    ), // option 287
    (
        [b'A', b'B', b'F', b'G', b'I', b'J', b'K', b'L'],
        [b'I', b'J', b'B', b'F', b'A', b'G', b'L', b'K'],
    ), // option 288
    (
        [b'A', b'B', b'F', b'G', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'F', b'A', b'G', b'L', b'K'],
    ), // option 289
    (
        [b'A', b'B', b'F', b'G', b'H', b'I', b'K', b'L'],
        [b'H', b'G', b'B', b'A', b'I', b'F', b'L', b'K'],
    ), // option 290
    (
        [b'A', b'B', b'F', b'G', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'B', b'F', b'A', b'G', b'L', b'I'],
    ), // option 291
    (
        [b'A', b'B', b'F', b'G', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'B', b'F', b'A', b'G', b'I', b'K'],
    ), // option 292
    (
        [b'A', b'B', b'E', b'H', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'A', b'I', b'H', b'L', b'K'],
    ), // option 293
    (
        [b'A', b'B', b'E', b'G', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'A', b'I', b'G', b'L', b'K'],
    ), // option 294
    (
        [b'A', b'B', b'E', b'G', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'A', b'H', b'G', b'L', b'K'],
    ), // option 295
    (
        [b'A', b'B', b'E', b'G', b'H', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'A', b'I', b'H', b'L', b'K'],
    ), // option 296
    (
        [b'A', b'B', b'E', b'G', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'A', b'H', b'G', b'L', b'I'],
    ), // option 297
    (
        [b'A', b'B', b'E', b'G', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'A', b'H', b'G', b'I', b'K'],
    ), // option 298
    (
        [b'A', b'B', b'E', b'F', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'A', b'I', b'F', b'L', b'K'],
    ), // option 299
    (
        [b'A', b'B', b'E', b'F', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'F', b'A', b'H', b'L', b'K'],
    ), // option 300
    (
        [b'A', b'B', b'E', b'F', b'H', b'I', b'K', b'L'],
        [b'E', b'I', b'B', b'F', b'A', b'H', b'L', b'K'],
    ), // option 301
    (
        [b'A', b'B', b'E', b'F', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'F', b'A', b'H', b'L', b'I'],
    ), // option 302
    (
        [b'A', b'B', b'E', b'F', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'F', b'A', b'H', b'I', b'K'],
    ), // option 303
    (
        [b'A', b'B', b'E', b'F', b'G', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'F', b'A', b'G', b'L', b'K'],
    ), // option 304
    (
        [b'A', b'B', b'E', b'F', b'G', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'A', b'I', b'F', b'L', b'K'],
    ), // option 305
    (
        [b'A', b'B', b'E', b'F', b'G', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'F', b'A', b'G', b'L', b'I'],
    ), // option 306
    (
        [b'A', b'B', b'E', b'F', b'G', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'F', b'A', b'G', b'I', b'K'],
    ), // option 307
    (
        [b'A', b'B', b'E', b'F', b'G', b'H', b'K', b'L'],
        [b'E', b'G', b'B', b'F', b'A', b'H', b'L', b'K'],
    ), // option 308
    (
        [b'A', b'B', b'E', b'F', b'G', b'H', b'J', b'L'],
        [b'H', b'J', b'B', b'F', b'A', b'G', b'L', b'E'],
    ), // option 309
    (
        [b'A', b'B', b'E', b'F', b'G', b'H', b'J', b'K'],
        [b'H', b'J', b'B', b'F', b'A', b'G', b'E', b'K'],
    ), // option 310
    (
        [b'A', b'B', b'E', b'F', b'G', b'H', b'I', b'L'],
        [b'E', b'G', b'B', b'F', b'A', b'H', b'L', b'I'],
    ), // option 311
    (
        [b'A', b'B', b'E', b'F', b'G', b'H', b'I', b'K'],
        [b'E', b'G', b'B', b'F', b'A', b'H', b'I', b'K'],
    ), // option 312
    (
        [b'A', b'B', b'E', b'F', b'G', b'H', b'I', b'J'],
        [b'H', b'J', b'B', b'F', b'A', b'G', b'E', b'I'],
    ), // option 313
    (
        [b'A', b'B', b'D', b'H', b'I', b'J', b'K', b'L'],
        [b'I', b'J', b'B', b'D', b'A', b'H', b'L', b'K'],
    ), // option 314
    (
        [b'A', b'B', b'D', b'G', b'I', b'J', b'K', b'L'],
        [b'I', b'J', b'B', b'D', b'A', b'G', b'L', b'K'],
    ), // option 315
    (
        [b'A', b'B', b'D', b'G', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'D', b'A', b'G', b'L', b'K'],
    ), // option 316
    (
        [b'A', b'B', b'D', b'G', b'H', b'I', b'K', b'L'],
        [b'I', b'G', b'B', b'D', b'A', b'H', b'L', b'K'],
    ), // option 317
    (
        [b'A', b'B', b'D', b'G', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'B', b'D', b'A', b'G', b'L', b'I'],
    ), // option 318
    (
        [b'A', b'B', b'D', b'G', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'B', b'D', b'A', b'G', b'I', b'K'],
    ), // option 319
    (
        [b'A', b'B', b'D', b'F', b'I', b'J', b'K', b'L'],
        [b'I', b'J', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 320
    (
        [b'A', b'B', b'D', b'F', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 321
    (
        [b'A', b'B', b'D', b'F', b'H', b'I', b'K', b'L'],
        [b'H', b'I', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 322
    (
        [b'A', b'B', b'D', b'F', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'B', b'D', b'A', b'F', b'L', b'I'],
    ), // option 323
    (
        [b'A', b'B', b'D', b'F', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'B', b'D', b'A', b'F', b'I', b'K'],
    ), // option 324
    (
        [b'A', b'B', b'D', b'F', b'G', b'J', b'K', b'L'],
        [b'F', b'J', b'B', b'D', b'A', b'G', b'L', b'K'],
    ), // option 325
    (
        [b'A', b'B', b'D', b'F', b'G', b'I', b'K', b'L'],
        [b'I', b'G', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 326
    (
        [b'A', b'B', b'D', b'F', b'G', b'I', b'J', b'L'],
        [b'F', b'J', b'B', b'D', b'A', b'G', b'L', b'I'],
    ), // option 327
    (
        [b'A', b'B', b'D', b'F', b'G', b'I', b'J', b'K'],
        [b'F', b'J', b'B', b'D', b'A', b'G', b'I', b'K'],
    ), // option 328
    (
        [b'A', b'B', b'D', b'F', b'G', b'H', b'K', b'L'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 329
    (
        [b'A', b'B', b'D', b'F', b'G', b'H', b'J', b'L'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'L', b'J'],
    ), // option 330
    (
        [b'A', b'B', b'D', b'F', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'J', b'K'],
    ), // option 331
    (
        [b'A', b'B', b'D', b'F', b'G', b'H', b'I', b'L'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'L', b'I'],
    ), // option 332
    (
        [b'A', b'B', b'D', b'F', b'G', b'H', b'I', b'K'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'I', b'K'],
    ), // option 333
    (
        [b'A', b'B', b'D', b'F', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'I', b'J'],
    ), // option 334
    (
        [b'A', b'B', b'D', b'E', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'A', b'I', b'D', b'L', b'K'],
    ), // option 335
    (
        [b'A', b'B', b'D', b'E', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'D', b'A', b'H', b'L', b'K'],
    ), // option 336
    (
        [b'A', b'B', b'D', b'E', b'H', b'I', b'K', b'L'],
        [b'E', b'I', b'B', b'D', b'A', b'H', b'L', b'K'],
    ), // option 337
    (
        [b'A', b'B', b'D', b'E', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'D', b'A', b'H', b'L', b'I'],
    ), // option 338
    (
        [b'A', b'B', b'D', b'E', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'D', b'A', b'H', b'I', b'K'],
    ), // option 339
    (
        [b'A', b'B', b'D', b'E', b'G', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'D', b'A', b'G', b'L', b'K'],
    ), // option 340
    (
        [b'A', b'B', b'D', b'E', b'G', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'A', b'I', b'D', b'L', b'K'],
    ), // option 341
    (
        [b'A', b'B', b'D', b'E', b'G', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'D', b'A', b'G', b'L', b'I'],
    ), // option 342
    (
        [b'A', b'B', b'D', b'E', b'G', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'D', b'A', b'G', b'I', b'K'],
    ), // option 343
    (
        [b'A', b'B', b'D', b'E', b'G', b'H', b'K', b'L'],
        [b'E', b'G', b'B', b'D', b'A', b'H', b'L', b'K'],
    ), // option 344
    (
        [b'A', b'B', b'D', b'E', b'G', b'H', b'J', b'L'],
        [b'H', b'J', b'B', b'D', b'A', b'G', b'L', b'E'],
    ), // option 345
    (
        [b'A', b'B', b'D', b'E', b'G', b'H', b'J', b'K'],
        [b'H', b'J', b'B', b'D', b'A', b'G', b'E', b'K'],
    ), // option 346
    (
        [b'A', b'B', b'D', b'E', b'G', b'H', b'I', b'L'],
        [b'E', b'G', b'B', b'D', b'A', b'H', b'L', b'I'],
    ), // option 347
    (
        [b'A', b'B', b'D', b'E', b'G', b'H', b'I', b'K'],
        [b'E', b'G', b'B', b'D', b'A', b'H', b'I', b'K'],
    ), // option 348
    (
        [b'A', b'B', b'D', b'E', b'G', b'H', b'I', b'J'],
        [b'H', b'J', b'B', b'D', b'A', b'G', b'E', b'I'],
    ), // option 349
    (
        [b'A', b'B', b'D', b'E', b'F', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 350
    (
        [b'A', b'B', b'D', b'E', b'F', b'I', b'K', b'L'],
        [b'E', b'I', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 351
    (
        [b'A', b'B', b'D', b'E', b'F', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'D', b'A', b'F', b'L', b'I'],
    ), // option 352
    (
        [b'A', b'B', b'D', b'E', b'F', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'D', b'A', b'F', b'I', b'K'],
    ), // option 353
    (
        [b'A', b'B', b'D', b'E', b'F', b'H', b'K', b'L'],
        [b'H', b'E', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 354
    (
        [b'A', b'B', b'D', b'E', b'F', b'H', b'J', b'L'],
        [b'H', b'J', b'B', b'D', b'A', b'F', b'L', b'E'],
    ), // option 355
    (
        [b'A', b'B', b'D', b'E', b'F', b'H', b'J', b'K'],
        [b'H', b'J', b'B', b'D', b'A', b'F', b'E', b'K'],
    ), // option 356
    (
        [b'A', b'B', b'D', b'E', b'F', b'H', b'I', b'L'],
        [b'H', b'E', b'B', b'D', b'A', b'F', b'L', b'I'],
    ), // option 357
    (
        [b'A', b'B', b'D', b'E', b'F', b'H', b'I', b'K'],
        [b'H', b'E', b'B', b'D', b'A', b'F', b'I', b'K'],
    ), // option 358
    (
        [b'A', b'B', b'D', b'E', b'F', b'H', b'I', b'J'],
        [b'H', b'J', b'B', b'D', b'A', b'F', b'E', b'I'],
    ), // option 359
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'K', b'L'],
        [b'E', b'G', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 360
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'J', b'L'],
        [b'E', b'G', b'B', b'D', b'A', b'F', b'L', b'J'],
    ), // option 361
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'J', b'K'],
        [b'E', b'G', b'B', b'D', b'A', b'F', b'J', b'K'],
    ), // option 362
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'I', b'L'],
        [b'E', b'G', b'B', b'D', b'A', b'F', b'L', b'I'],
    ), // option 363
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'I', b'K'],
        [b'E', b'G', b'B', b'D', b'A', b'F', b'I', b'K'],
    ), // option 364
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'I', b'J'],
        [b'E', b'G', b'B', b'D', b'A', b'F', b'I', b'J'],
    ), // option 365
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'H', b'L'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'L', b'E'],
    ), // option 366
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'H', b'K'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'E', b'K'],
    ), // option 367
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'H', b'J'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'E', b'J'],
    ), // option 368
    (
        [b'A', b'B', b'D', b'E', b'F', b'G', b'H', b'I'],
        [b'H', b'G', b'B', b'D', b'A', b'F', b'E', b'I'],
    ), // option 369
    (
        [b'A', b'B', b'C', b'H', b'I', b'J', b'K', b'L'],
        [b'I', b'J', b'B', b'C', b'A', b'H', b'L', b'K'],
    ), // option 370
    (
        [b'A', b'B', b'C', b'G', b'I', b'J', b'K', b'L'],
        [b'I', b'J', b'B', b'C', b'A', b'G', b'L', b'K'],
    ), // option 371
    (
        [b'A', b'B', b'C', b'G', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'C', b'A', b'G', b'L', b'K'],
    ), // option 372
    (
        [b'A', b'B', b'C', b'G', b'H', b'I', b'K', b'L'],
        [b'I', b'G', b'B', b'C', b'A', b'H', b'L', b'K'],
    ), // option 373
    (
        [b'A', b'B', b'C', b'G', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'B', b'C', b'A', b'G', b'L', b'I'],
    ), // option 374
    (
        [b'A', b'B', b'C', b'G', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'B', b'C', b'A', b'G', b'I', b'K'],
    ), // option 375
    (
        [b'A', b'B', b'C', b'F', b'I', b'J', b'K', b'L'],
        [b'I', b'J', b'B', b'C', b'A', b'F', b'L', b'K'],
    ), // option 376
    (
        [b'A', b'B', b'C', b'F', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'C', b'A', b'F', b'L', b'K'],
    ), // option 377
    (
        [b'A', b'B', b'C', b'F', b'H', b'I', b'K', b'L'],
        [b'H', b'I', b'B', b'C', b'A', b'F', b'L', b'K'],
    ), // option 378
    (
        [b'A', b'B', b'C', b'F', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'B', b'C', b'A', b'F', b'L', b'I'],
    ), // option 379
    (
        [b'A', b'B', b'C', b'F', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'B', b'C', b'A', b'F', b'I', b'K'],
    ), // option 380
    (
        [b'A', b'B', b'C', b'F', b'G', b'J', b'K', b'L'],
        [b'C', b'J', b'B', b'F', b'A', b'G', b'L', b'K'],
    ), // option 381
    (
        [b'A', b'B', b'C', b'F', b'G', b'I', b'K', b'L'],
        [b'I', b'G', b'B', b'C', b'A', b'F', b'L', b'K'],
    ), // option 382
    (
        [b'A', b'B', b'C', b'F', b'G', b'I', b'J', b'L'],
        [b'C', b'J', b'B', b'F', b'A', b'G', b'L', b'I'],
    ), // option 383
    (
        [b'A', b'B', b'C', b'F', b'G', b'I', b'J', b'K'],
        [b'C', b'J', b'B', b'F', b'A', b'G', b'I', b'K'],
    ), // option 384
    (
        [b'A', b'B', b'C', b'F', b'G', b'H', b'K', b'L'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'L', b'K'],
    ), // option 385
    (
        [b'A', b'B', b'C', b'F', b'G', b'H', b'J', b'L'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'L', b'J'],
    ), // option 386
    (
        [b'A', b'B', b'C', b'F', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'J', b'K'],
    ), // option 387
    (
        [b'A', b'B', b'C', b'F', b'G', b'H', b'I', b'L'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'L', b'I'],
    ), // option 388
    (
        [b'A', b'B', b'C', b'F', b'G', b'H', b'I', b'K'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'I', b'K'],
    ), // option 389
    (
        [b'A', b'B', b'C', b'F', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'I', b'J'],
    ), // option 390
    (
        [b'A', b'B', b'C', b'E', b'I', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'A', b'I', b'C', b'L', b'K'],
    ), // option 391
    (
        [b'A', b'B', b'C', b'E', b'H', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'A', b'H', b'L', b'K'],
    ), // option 392
    (
        [b'A', b'B', b'C', b'E', b'H', b'I', b'K', b'L'],
        [b'E', b'I', b'B', b'C', b'A', b'H', b'L', b'K'],
    ), // option 393
    (
        [b'A', b'B', b'C', b'E', b'H', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'C', b'A', b'H', b'L', b'I'],
    ), // option 394
    (
        [b'A', b'B', b'C', b'E', b'H', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'C', b'A', b'H', b'I', b'K'],
    ), // option 395
    (
        [b'A', b'B', b'C', b'E', b'G', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'A', b'G', b'L', b'K'],
    ), // option 396
    (
        [b'A', b'B', b'C', b'E', b'G', b'I', b'K', b'L'],
        [b'E', b'G', b'B', b'A', b'I', b'C', b'L', b'K'],
    ), // option 397
    (
        [b'A', b'B', b'C', b'E', b'G', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'C', b'A', b'G', b'L', b'I'],
    ), // option 398
    (
        [b'A', b'B', b'C', b'E', b'G', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'C', b'A', b'G', b'I', b'K'],
    ), // option 399
    (
        [b'A', b'B', b'C', b'E', b'G', b'H', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'A', b'H', b'L', b'K'],
    ), // option 400
    (
        [b'A', b'B', b'C', b'E', b'G', b'H', b'J', b'L'],
        [b'H', b'J', b'B', b'C', b'A', b'G', b'L', b'E'],
    ), // option 401
    (
        [b'A', b'B', b'C', b'E', b'G', b'H', b'J', b'K'],
        [b'H', b'J', b'B', b'C', b'A', b'G', b'E', b'K'],
    ), // option 402
    (
        [b'A', b'B', b'C', b'E', b'G', b'H', b'I', b'L'],
        [b'E', b'G', b'B', b'C', b'A', b'H', b'L', b'I'],
    ), // option 403
    (
        [b'A', b'B', b'C', b'E', b'G', b'H', b'I', b'K'],
        [b'E', b'G', b'B', b'C', b'A', b'H', b'I', b'K'],
    ), // option 404
    (
        [b'A', b'B', b'C', b'E', b'G', b'H', b'I', b'J'],
        [b'H', b'J', b'B', b'C', b'A', b'G', b'E', b'I'],
    ), // option 405
    (
        [b'A', b'B', b'C', b'E', b'F', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'A', b'F', b'L', b'K'],
    ), // option 406
    (
        [b'A', b'B', b'C', b'E', b'F', b'I', b'K', b'L'],
        [b'E', b'I', b'B', b'C', b'A', b'F', b'L', b'K'],
    ), // option 407
    (
        [b'A', b'B', b'C', b'E', b'F', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'C', b'A', b'F', b'L', b'I'],
    ), // option 408
    (
        [b'A', b'B', b'C', b'E', b'F', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'C', b'A', b'F', b'I', b'K'],
    ), // option 409
    (
        [b'A', b'B', b'C', b'E', b'F', b'H', b'K', b'L'],
        [b'H', b'E', b'B', b'C', b'A', b'F', b'L', b'K'],
    ), // option 410
    (
        [b'A', b'B', b'C', b'E', b'F', b'H', b'J', b'L'],
        [b'H', b'J', b'B', b'C', b'A', b'F', b'L', b'E'],
    ), // option 411
    (
        [b'A', b'B', b'C', b'E', b'F', b'H', b'J', b'K'],
        [b'H', b'J', b'B', b'C', b'A', b'F', b'E', b'K'],
    ), // option 412
    (
        [b'A', b'B', b'C', b'E', b'F', b'H', b'I', b'L'],
        [b'H', b'E', b'B', b'C', b'A', b'F', b'L', b'I'],
    ), // option 413
    (
        [b'A', b'B', b'C', b'E', b'F', b'H', b'I', b'K'],
        [b'H', b'E', b'B', b'C', b'A', b'F', b'I', b'K'],
    ), // option 414
    (
        [b'A', b'B', b'C', b'E', b'F', b'H', b'I', b'J'],
        [b'H', b'J', b'B', b'C', b'A', b'F', b'E', b'I'],
    ), // option 415
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'A', b'F', b'L', b'K'],
    ), // option 416
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'J', b'L'],
        [b'E', b'G', b'B', b'C', b'A', b'F', b'L', b'J'],
    ), // option 417
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'J', b'K'],
        [b'E', b'G', b'B', b'C', b'A', b'F', b'J', b'K'],
    ), // option 418
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'I', b'L'],
        [b'E', b'G', b'B', b'C', b'A', b'F', b'L', b'I'],
    ), // option 419
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'I', b'K'],
        [b'E', b'G', b'B', b'C', b'A', b'F', b'I', b'K'],
    ), // option 420
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'I', b'J'],
        [b'E', b'G', b'B', b'C', b'A', b'F', b'I', b'J'],
    ), // option 421
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'H', b'L'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'L', b'E'],
    ), // option 422
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'H', b'K'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'E', b'K'],
    ), // option 423
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'H', b'J'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'E', b'J'],
    ), // option 424
    (
        [b'A', b'B', b'C', b'E', b'F', b'G', b'H', b'I'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'E', b'I'],
    ), // option 425
    (
        [b'A', b'B', b'C', b'D', b'I', b'J', b'K', b'L'],
        [b'I', b'J', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 426
    (
        [b'A', b'B', b'C', b'D', b'H', b'J', b'K', b'L'],
        [b'H', b'J', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 427
    (
        [b'A', b'B', b'C', b'D', b'H', b'I', b'K', b'L'],
        [b'H', b'I', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 428
    (
        [b'A', b'B', b'C', b'D', b'H', b'I', b'J', b'L'],
        [b'H', b'J', b'B', b'C', b'A', b'D', b'L', b'I'],
    ), // option 429
    (
        [b'A', b'B', b'C', b'D', b'H', b'I', b'J', b'K'],
        [b'H', b'J', b'B', b'C', b'A', b'D', b'I', b'K'],
    ), // option 430
    (
        [b'A', b'B', b'C', b'D', b'G', b'J', b'K', b'L'],
        [b'C', b'J', b'B', b'D', b'A', b'G', b'L', b'K'],
    ), // option 431
    (
        [b'A', b'B', b'C', b'D', b'G', b'I', b'K', b'L'],
        [b'I', b'G', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 432
    (
        [b'A', b'B', b'C', b'D', b'G', b'I', b'J', b'L'],
        [b'C', b'J', b'B', b'D', b'A', b'G', b'L', b'I'],
    ), // option 433
    (
        [b'A', b'B', b'C', b'D', b'G', b'I', b'J', b'K'],
        [b'C', b'J', b'B', b'D', b'A', b'G', b'I', b'K'],
    ), // option 434
    (
        [b'A', b'B', b'C', b'D', b'G', b'H', b'K', b'L'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 435
    (
        [b'A', b'B', b'C', b'D', b'G', b'H', b'J', b'L'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'L', b'J'],
    ), // option 436
    (
        [b'A', b'B', b'C', b'D', b'G', b'H', b'J', b'K'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'J', b'K'],
    ), // option 437
    (
        [b'A', b'B', b'C', b'D', b'G', b'H', b'I', b'L'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'L', b'I'],
    ), // option 438
    (
        [b'A', b'B', b'C', b'D', b'G', b'H', b'I', b'K'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'I', b'K'],
    ), // option 439
    (
        [b'A', b'B', b'C', b'D', b'G', b'H', b'I', b'J'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'I', b'J'],
    ), // option 440
    (
        [b'A', b'B', b'C', b'D', b'F', b'J', b'K', b'L'],
        [b'C', b'J', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 441
    (
        [b'A', b'B', b'C', b'D', b'F', b'I', b'K', b'L'],
        [b'C', b'I', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 442
    (
        [b'A', b'B', b'C', b'D', b'F', b'I', b'J', b'L'],
        [b'C', b'J', b'B', b'D', b'A', b'F', b'L', b'I'],
    ), // option 443
    (
        [b'A', b'B', b'C', b'D', b'F', b'I', b'J', b'K'],
        [b'C', b'J', b'B', b'D', b'A', b'F', b'I', b'K'],
    ), // option 444
    (
        [b'A', b'B', b'C', b'D', b'F', b'H', b'K', b'L'],
        [b'H', b'F', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 445
    (
        [b'A', b'B', b'C', b'D', b'F', b'H', b'J', b'L'],
        [b'C', b'J', b'B', b'D', b'A', b'F', b'L', b'H'],
    ), // option 446
    (
        [b'A', b'B', b'C', b'D', b'F', b'H', b'J', b'K'],
        [b'H', b'J', b'B', b'C', b'A', b'F', b'D', b'K'],
    ), // option 447
    (
        [b'A', b'B', b'C', b'D', b'F', b'H', b'I', b'L'],
        [b'H', b'F', b'B', b'C', b'A', b'D', b'L', b'I'],
    ), // option 448
    (
        [b'A', b'B', b'C', b'D', b'F', b'H', b'I', b'K'],
        [b'H', b'F', b'B', b'C', b'A', b'D', b'I', b'K'],
    ), // option 449
    (
        [b'A', b'B', b'C', b'D', b'F', b'H', b'I', b'J'],
        [b'H', b'J', b'B', b'C', b'A', b'F', b'D', b'I'],
    ), // option 450
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'K', b'L'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 451
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'J', b'L'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'L', b'J'],
    ), // option 452
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'J', b'K'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'J', b'K'],
    ), // option 453
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'I', b'L'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'L', b'I'],
    ), // option 454
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'I', b'K'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'I', b'K'],
    ), // option 455
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'I', b'J'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'I', b'J'],
    ), // option 456
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'H', b'L'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'L', b'H'],
    ), // option 457
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'H', b'K'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'D', b'K'],
    ), // option 458
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'H', b'J'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'D', b'J'],
    ), // option 459
    (
        [b'A', b'B', b'C', b'D', b'F', b'G', b'H', b'I'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'D', b'I'],
    ), // option 460
    (
        [b'A', b'B', b'C', b'D', b'E', b'J', b'K', b'L'],
        [b'E', b'J', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 461
    (
        [b'A', b'B', b'C', b'D', b'E', b'I', b'K', b'L'],
        [b'E', b'I', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 462
    (
        [b'A', b'B', b'C', b'D', b'E', b'I', b'J', b'L'],
        [b'E', b'J', b'B', b'C', b'A', b'D', b'L', b'I'],
    ), // option 463
    (
        [b'A', b'B', b'C', b'D', b'E', b'I', b'J', b'K'],
        [b'E', b'J', b'B', b'C', b'A', b'D', b'I', b'K'],
    ), // option 464
    (
        [b'A', b'B', b'C', b'D', b'E', b'H', b'K', b'L'],
        [b'H', b'E', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 465
    (
        [b'A', b'B', b'C', b'D', b'E', b'H', b'J', b'L'],
        [b'H', b'J', b'B', b'C', b'A', b'D', b'L', b'E'],
    ), // option 466
    (
        [b'A', b'B', b'C', b'D', b'E', b'H', b'J', b'K'],
        [b'H', b'J', b'B', b'C', b'A', b'D', b'E', b'K'],
    ), // option 467
    (
        [b'A', b'B', b'C', b'D', b'E', b'H', b'I', b'L'],
        [b'H', b'E', b'B', b'C', b'A', b'D', b'L', b'I'],
    ), // option 468
    (
        [b'A', b'B', b'C', b'D', b'E', b'H', b'I', b'K'],
        [b'H', b'E', b'B', b'C', b'A', b'D', b'I', b'K'],
    ), // option 469
    (
        [b'A', b'B', b'C', b'D', b'E', b'H', b'I', b'J'],
        [b'H', b'J', b'B', b'C', b'A', b'D', b'E', b'I'],
    ), // option 470
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'K', b'L'],
        [b'E', b'G', b'B', b'C', b'A', b'D', b'L', b'K'],
    ), // option 471
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'J', b'L'],
        [b'E', b'G', b'B', b'C', b'A', b'D', b'L', b'J'],
    ), // option 472
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'J', b'K'],
        [b'E', b'G', b'B', b'C', b'A', b'D', b'J', b'K'],
    ), // option 473
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'I', b'L'],
        [b'E', b'G', b'B', b'C', b'A', b'D', b'L', b'I'],
    ), // option 474
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'I', b'K'],
        [b'E', b'G', b'B', b'C', b'A', b'D', b'I', b'K'],
    ), // option 475
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'I', b'J'],
        [b'E', b'G', b'B', b'C', b'A', b'D', b'I', b'J'],
    ), // option 476
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'H', b'L'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'L', b'E'],
    ), // option 477
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'H', b'K'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'E', b'K'],
    ), // option 478
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'H', b'J'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'E', b'J'],
    ), // option 479
    (
        [b'A', b'B', b'C', b'D', b'E', b'G', b'H', b'I'],
        [b'H', b'G', b'B', b'C', b'A', b'D', b'E', b'I'],
    ), // option 480
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'K', b'L'],
        [b'C', b'E', b'B', b'D', b'A', b'F', b'L', b'K'],
    ), // option 481
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'J', b'L'],
        [b'C', b'J', b'B', b'D', b'A', b'F', b'L', b'E'],
    ), // option 482
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'J', b'K'],
        [b'C', b'J', b'B', b'D', b'A', b'F', b'E', b'K'],
    ), // option 483
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'I', b'L'],
        [b'C', b'E', b'B', b'D', b'A', b'F', b'L', b'I'],
    ), // option 484
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'I', b'K'],
        [b'C', b'E', b'B', b'D', b'A', b'F', b'I', b'K'],
    ), // option 485
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'I', b'J'],
        [b'C', b'J', b'B', b'D', b'A', b'F', b'E', b'I'],
    ), // option 486
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'H', b'L'],
        [b'H', b'F', b'B', b'C', b'A', b'D', b'L', b'E'],
    ), // option 487
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'H', b'K'],
        [b'H', b'E', b'B', b'C', b'A', b'F', b'D', b'K'],
    ), // option 488
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'H', b'J'],
        [b'H', b'J', b'B', b'C', b'A', b'F', b'D', b'E'],
    ), // option 489
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'H', b'I'],
        [b'H', b'E', b'B', b'C', b'A', b'F', b'D', b'I'],
    ), // option 490
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'L'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'L', b'E'],
    ), // option 491
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'K'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'E', b'K'],
    ), // option 492
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'J'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'E', b'J'],
    ), // option 493
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'I'],
        [b'C', b'G', b'B', b'D', b'A', b'F', b'E', b'I'],
    ), // option 494
    (
        [b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H'],
        [b'H', b'G', b'B', b'C', b'A', b'F', b'D', b'E'],
    ), // option 495
];
