// (c) 2019, David Freese, W1HKJ

#ifndef __FIFO_H__
#define __FIFO_H__

#include "pj_struc.h"

// a simple FIFO buffer
template <class Type>
class FIFO {
public:
	size_t Len;
private:
	size_t ReadPtr;
	size_t WritePtr;
	Type *Data;

public:
	FIFO() { Init(); }
	~FIFO() { delete [] Data; }

	void Init(void) { Data = 0; Len = 0; }

	void Free(void) { delete [] Data; Data = 0; Len = 0; }

	void Reset(void) {
		ReadPtr = WritePtr = 0;
	}

	void Clear(void) {
		ReadPtr = WritePtr;
	}

	int Preset(void) {
		if (ReallocArray(&Data, Len) < 0)
			return -1;
		Reset();
		return 0;
	}

	// increment the pointer (with wrapping around)
	void IncrPtr(size_t &Ptr, size_t Step = 1) {
		Ptr += Step;
		if (Ptr >= Len)
			Ptr-=Len;
	}

	// FIFO is full ?
	int Full(void) {
		size_t Ptr = WritePtr;
		IncrPtr(Ptr);
		return (Ptr == ReadPtr);
	}

	// FIFO is empty ?
	int Empty(void) {
		return (ReadPtr == WritePtr);
	}

	// how many elements we can write = space left in the FIFO
	size_t WriteReady(void) {
		int Ready = ReadPtr - WritePtr;
		if (Ready <= 0) Ready += Len;
		return Ready - 1;
	}

	// how many elements we can read = space taken in the FIFO
	size_t ReadReady(void) {
		int Ready = WritePtr - ReadPtr;
		if (Ready < 0)
			Ready += Len;
		return Ready;
	}

	// write a new element
	int Write(Type &NewData) {
		size_t Ptr = WritePtr;
		IncrPtr(Ptr);
		if (Ptr == ReadPtr)
			return 0;
		Data[WritePtr] = NewData;
		WritePtr = Ptr;
		return 1;
	}

	// read the oldest element
	int Read(Type &OldData) {
		if (ReadPtr == WritePtr)
			return 0;
		OldData = Data[ReadPtr];
		IncrPtr(ReadPtr);
		return 1;
	}

	// lookup data in the FIFO but without taking them out
	int Lookup(Type &OldData, size_t Offset = 0) {
		size_t Ready = ReadReady();
		if (Offset >= Ready)
			return 0;
		size_t Ptr = ReadPtr;
		IncrPtr(Ptr,Offset);
		OldData = Data[Ptr];
		return 1;
	}
};

#endif // of __FIFO_H__
