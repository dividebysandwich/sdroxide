// Fast Hadamard Transform, Pawel Jalocha, December 2004
// (c) 1999-2003, Pawel Jalocha
// (c) 2019, David Freese, W1HKJ

#ifndef __FHT_H__
#define __FHT_H__

// Forward Fast Hadamard Transform

template <class Type>  // Type can be float, Cmpx<>, int8_t, etc.
void FHT(Type *Data, size_t Len) {
	size_t Step, Ptr, Ptr2; Type Bit1, Bit2, NewBit1, NewBit2;
	for (Step = 1; Step < Len; Step *= 2) {
		for (Ptr = 0; Ptr < Len; Ptr += 2 * Step) {
			for (Ptr2 = Ptr; (Ptr2 - Ptr) < Step; Ptr2 += 1) {
				Bit1 = Data[Ptr2];
				Bit2 = Data[Ptr2 + Step];
				NewBit1 = Bit2;
				NewBit1 += Bit1;
				NewBit2 = Bit2;
				NewBit2 -= Bit1;
				Data[Ptr2] = NewBit1;
				Data[Ptr2 + Step] = NewBit2;
			}
		}
	}
}

// Inverse Fast Hadamard Transform

template <class Type>
void IFHT(Type *Data, size_t Len) {
	size_t Step, Ptr, Ptr2;
	Type Bit1, Bit2, NewBit1, NewBit2;
	for (Step = Len/2; Step; Step /= 2) {
		for (Ptr = 0; Ptr < Len; Ptr += 2 * Step) {
			for (Ptr2 = Ptr; (Ptr2 - Ptr) < Step; Ptr2 += 1) {
				Bit1 = Data[Ptr2];
				Bit2 = Data[Ptr2 + Step];
				NewBit1 = Bit1;
				NewBit1 -= Bit2;
				NewBit2 = Bit1;
				NewBit2 += Bit2;
				Data[Ptr2] = NewBit1;
				Data[Ptr2 + Step] = NewBit2;
			}
		}
	}
}

#endif // of __FHT_H__

