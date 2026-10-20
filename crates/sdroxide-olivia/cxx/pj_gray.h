// Gray code convertion
// (c) 2004, Pawel Jalocha
// (c) 2019, David Freese, W1HKJ

#ifndef __GRAY_H__
#define __GRAY_H__

#include <stdint.h>

template <class Type>
Type GrayCode(Type Binary) {
	return Binary ^ (Binary >> 1);
}

inline uint8_t BinaryCode(uint8_t Gray) {
	Gray ^= (Gray >> 4);
	Gray ^= (Gray >> 2);
	Gray ^= (Gray >> 1);
	return Gray;
}

inline uint16_t BinaryCode(uint16_t Gray) {
	Gray ^= (Gray >> 8);
	Gray ^= (Gray >> 4);
	Gray ^= (Gray >> 2);
	Gray ^= (Gray >> 1);
	return Gray;
}

inline uint32_t BinaryCode(uint32_t Gray) {
	Gray ^= (Gray >> 16);
	Gray ^= (Gray >> 8);
	Gray ^= (Gray >> 4);
	Gray ^= (Gray >> 2);
	Gray ^= (Gray >> 1);
	return Gray;
	}

#endif
