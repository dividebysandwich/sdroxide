// Fast Fourier Transform
// (c) 1999-2004, Pawel Jalocha

#ifndef __FFT_H__
#define __FFT_H__

#include <math.h>

#include "pj_cmpx.h"
#include "pj_struc.h"

// ----------------------------------------------------------------------------

/*
How to use the r2FFT class:

1. define the object:              r2FFT<dcmpx> FFT;

2. preset it for given FFT length: ret=FFT.Preset(1024);
	if return code is negative => your RAM is out, you can't use the FFT object.
	- after FFT.Preset() you have the unscrambling table in FFT.BitRevIdx[]
	  and the full (co)sine table in FFT.Twiddle[]

3. for forward complex FFT of "Data": FFT.Process(Data);
	(this includes unscrambling)

4. for inverse complex FFT of "Data":
	- first: negate the imaginary part of "Data"
	- second: execute FFT.Process(Data);
	- third: negate (again) the imaginary part of "Data"

New feature: you can call FFT.Process(Data,Len);

5. You may call FFT.Free() to free allocated RAM, but you will need
	to call FFT.Preset() before using the FFT object again.

6. Scaling: each pass of the FFT.Process(Data) multiplies the energy
	of the sequence contained in "Data" by the length of the FFT.
	To get the same scale (amplitude) after executing one forward
	and another inverse FFT you need to multiply the sequence by 1/length.

7. To make an FFT of two _real_ time sequences in one go:
	- place the first time sequence in the real part of complex "Data"
	- place the second time sequence in the imaginary part of "Data"
	- execute: FFT.Process(Data);
	- execute: FFT.SeparTwoReals(Data,Spectr1,Spectr2);
	  where Spectr1/2 are complex arrays half the size of the FFT length.
	  Spectr1/2 contains now the complex FFT result
	  for the first and second real input sequence
	- Scaling: the sequence energy is multiplied by FFT length

8. To execute an Inverse FFT as to get two real sequences out of
	two spectral data:
	- have the two freq. sequeces in Spectr1/2
	- execute: FFT.JoinTwoReals(Spectr1,Spect2,Data);
	- execute: FFT.Process(Data);
	- Data[].Re contains the first time sequence
	- Data[].Im contains the second time sequence
	- Spectr1/2 and Data arrays are like for SeparTwoReals()
	- Scaling: the sequence energy is multiplied by _twice_ the FFT length
*/

template <class Type>
class r2FFT {

public:   // size must a power of 2: 2,4,8,16,32,64,128,256,...

	r2FFT(int MaxSize) {
		BitRevIdx = 0;
		Twiddle = 0;
		Preset(MaxSize);
	}

	r2FFT() {
		BitRevIdx = 0;
		Twiddle = 0;
	}

	~r2FFT() {
		delete [] BitRevIdx; //free(BitRevIdx);
		delete [] Twiddle; //free(Twiddle);
	}

	void Free(void) {
		delete [] BitRevIdx; //free(BitRevIdx);
		delete [] Twiddle; //free(Twiddle);
		BitRevIdx = 0;
		Twiddle = 0;
	}

	// preset tables for given (maximum) processing size
	int Preset(int MaxSize) {
		size_t idx, ridx, mask, rmask;
		double phase;
		size_t Size4;
		if (MaxSize < 4) goto Error;  
		Size = MaxSize;
		while ((MaxSize & 1) == 0)
			MaxSize >>= 1;
		if (MaxSize != 1) goto Error;

		if (ReallocArray(&BitRevIdx, Size) < 0) goto Error;
		if (ReallocArray(&Twiddle,Size) < 0) goto Error;

		Size4 = Size/4;
		for (idx = 0; idx < Size4; idx++) {
			phase = (2 * M_PI * idx)/Size;
			Twiddle[idx].SetPhase(phase);
		}
		for ( ; idx < Size; idx++) {
			Twiddle[idx].Re = (-Twiddle[idx - Size4].Im);
			Twiddle[idx].Im = Twiddle[idx - Size4].Re;
		}
		for (ridx = 0, idx=0; idx < Size; idx++) {
			for (ridx = 0, mask = Size/2, rmask = 1; mask; mask >>= 1, rmask <<= 1) {
				if (idx & mask) ridx |= rmask;
			}
			BitRevIdx[idx] = ridx;
			/* printf("%04x %04x\n",idx,ridx); */
		}
		return 0;
		Error:
			Free();
			return -1;
	}

	// scramble/unscramble (I)FFT input
	template <class DataType>
	void Scramble(DataType x[]) {
		size_t idx, ridx;
		DataType tmp;
		for (idx = 0; idx < Size; idx++) {
			if ((ridx = BitRevIdx[idx]) > idx) {
				tmp = x[idx];
				x[idx] = x[ridx];
				x[ridx] = tmp;
				/* printf("%d <=> %d\n",idx,ridx); */
			}
		}
	}

	template <class DataType>
	void Scramble(DataType x[], size_t ShrinkShift) {
		size_t idx,ridx;
		DataType tmp;
		size_t Len = Size >> ShrinkShift;
		for (idx = 0; idx < Len; idx++) {
			ridx = BitRevIdx[idx];
			ridx >>= ShrinkShift;
			if (ridx > idx) {
				tmp = x[idx];
				x[idx] = x[ridx];
				x[ridx] = tmp;
				/* printf("%d <=> %d\n",idx,ridx); */
			}
		}
	}

	// separate the result of a two real channels FFT
	template <class BuffType, class DataType>
	void SeparTwoReals(BuffType Buff[], DataType Out0[], DataType Out1[]) {
		int idx, HalfSize = Size/2;
	//	for(idx=0; idx<Size; idx++)
	//		printf("%2d %9.5f %9.5f\n",idx,Buff[idx].Re,Buff[idx].Im);
		Out0[0].Re = Buff[0].Re;
		Out1[0].Re = Buff[0].Im;
		for (idx = 1; idx < HalfSize; idx++) {
			Out0[idx].Re = Buff[idx].Re + Buff[Size - idx].Re;
			Out0[idx].Im = Buff[idx].Im - Buff[Size - idx].Im;
			Out1[idx].Re = Buff[idx].Im + Buff[Size - idx].Im;
			Out1[idx].Im = (-Buff[idx].Re) + Buff[Size - idx].Re;
		}
		Out0[0].Im = Buff[HalfSize].Re;
		Out1[0].Im = Buff[HalfSize].Im;
	//	for(idx=0; idx<HalfSize; idx++)
	//		printf("%2d  %9.5f %9.5f  %9.5f %9.5f\n",
	//			idx,Out0[idx].Re,Out0[idx].Im,Out1[idx].Re,Out1[idx].Im);  
	}

	// the oposite of SeparTwoReals()
	// but we NEGATE the .Im part for Inverse FFT  
	// and we NEGATE the Inp1[] so that after Process()
	// both .Re and .Im come out right (no need to negate the .Im part)
	// as a "by-product" we multiply the transform by 2
	template <class BuffType, class DataType>
	void JoinTwoReals(DataType Inp0[], DataType Inp1[], BuffType Buff[]) {
		int idx, HalfSize = Size/2;
	//	for(idx=0; idx<HalfSize; idx++)
	//		printf("%2d  %9.5f %9.5f  %9.5f %9.5f\n",
	//			idx,Inp0[idx].Re,Inp0[idx].Im,Inp1[idx].Re,Inp1[idx].Im);
		Buff[0].Re = 2 * Inp0[0].Re;
		Buff[0].Im = (2 * Inp1[0].Re);
		for (idx = 1; idx < HalfSize; idx++) {
			Buff[idx].Re      =  Inp0[idx].Re +Inp1[idx].Im;
			Buff[idx].Im      = (-Inp0[idx].Im)+Inp1[idx].Re;
			Buff[Size-idx].Re = Inp0[idx].Re - Inp1[idx].Im;
			Buff[Size-idx].Im = Inp0[idx].Im + Inp1[idx].Re;
		}
		Buff[HalfSize].Re = 2 * Inp0[0].Im;
		Buff[HalfSize].Im = 2 * Inp1[0].Im;
	//	for(idx=0; idx<Size; idx++)
	//		printf("%2d %9.5f %9.5f\n",idx,Buff[idx].Re,Buff[idx].Im);  
	}

	// core process: the classic tripple loop of butterflies
	// radix-2 FFT: the first and the second pass are by hand 
	// looks like there is no gain by separating the second pass
	// and even the first pass is in question ?
	template <class BuffType>
	void CoreProc(BuffType x[]) {
		size_t Groups, GroupSize2, Group, Bf, TwidIdx;
		size_t Size2 = Size/2;
		for (Bf = 0; Bf < Size; Bf += 2)
			FFT2(x[Bf], x[Bf + 1]); // first pass
		// for(Bf=0; Bf<Size; Bf+=4) FFT4(x[Bf],x[Bf+1],x[Bf+2],x[Bf+3]); // second
		// for(Groups=Size2/4,GroupSize2=4; Groups; Groups>>=1, GroupSize2<<=1)
		for (Groups = Size2/2, GroupSize2 = 2; Groups; Groups >>= 1, GroupSize2 <<= 1) {
			for (Group = 0, Bf = 0; Group < Groups; Group++, Bf += GroupSize2) {
				for (TwidIdx = 0; TwidIdx < Size2; TwidIdx += Groups, Bf++) {
					FFTbf(x[Bf], x[Bf + GroupSize2], Twiddle[TwidIdx]);
				}
			}
		}
	}

	// radix-2 FFT with a "shrink" factor
	template <class BuffType>
	void CoreProc(BuffType x[], size_t ShrinkShift) {
		size_t Groups, GroupSize2, Group, Bf, TwidIdx, TwidIncr;
		size_t Len = Size >> ShrinkShift;
		size_t Size2 = Size/2;
		size_t Len2 = Len/2;
		for (Bf = 0; Bf < Len; Bf += 2)
			FFT2(x[Bf], x[Bf + 1]); // first pass
		// for(Bf=0; Bf<Len; Bf+=4) FFT4(x[Bf],x[Bf+1],x[Bf+2],x[Bf+3]); // second
		for (Groups = Len2/2, TwidIncr = Size2/2, GroupSize2 = 2;
			 Groups;
			 Groups >>= 1, TwidIncr >>= 1, GroupSize2 <<= 1) {
			for (Group = 0, Bf = 0; Group < Groups; Group++, Bf += GroupSize2) {
				for (TwidIdx = 0; TwidIdx < Size2; TwidIdx += TwidIncr, Bf++) {
					FFTbf(x[Bf], x[Bf + GroupSize2], Twiddle[TwidIdx]);
				}
			}
		}
	}

	// complex FFT process in place, includes unscrambling
	template <class BuffType>
	 int Process(BuffType x[])
	  { Scramble(x); CoreProc(x); return 0; }

	// find the "shrink" factor for processing batches smaller than declared by Preset()
	int FindShrinkShift(size_t Len)
	  { size_t Shift;
		 for(Shift=0; Len<Size; Shift++)
			Len<<=1;
		 if (Len!=Size) return -1;
		 return Shift; }

	// process data with length smaller than requested by Preset() (but still a power of 2)
	template <class BuffType>
	 int Process(BuffType x[], size_t Len)
	  { if(Len<4) return -1;
		 if(Len==Size) { Scramble(x); CoreProc(x); return 0; }
		 int ShrinkShift=FindShrinkShift(Len); if(ShrinkShift<0) return -1;
		 Scramble(x,ShrinkShift); CoreProc(x,ShrinkShift); return 0; }

public:
	size_t Size;	        // FFT size (needs to be power of 2)
	size_t *BitRevIdx;	// Bit-reverse indexing table for data (un)scrambling
	Type *Twiddle;	// Twiddle factors (sine/cos values)

private:

	// classic radix-2 butterflies
	template <class BuffType>
	inline void FFTbf(BuffType &x0, BuffType &x1, Type &W) {
		Type x1W;
		x1W.Re = x1.Re*W.Re + x1.Im*W.Im;    // x1W.Re=x1.Re*W.Re-x1.Im*W.Im;
		x1W.Im = (-x1.Re*W.Im) + x1.Im*W.Re; // x1W.Im=x1.Re*W.Im+x1.Im*W.Re;
		x1.Re = x0.Re - x1W.Re;
		x1.Im = x0.Im - x1W.Im;
		x0.Re = x0.Re + x1W.Re;
		x0.Im = x0.Im + x1W.Im;
	}

	// special 2-point FFT for the first pass
	template <class BuffType>
	inline void FFT2(BuffType &x0, BuffType &x1) {
		Type x1W;
		x1W.Re = x1.Re;
		x1W.Im = x1.Im;
		x1.Re = x0.Re - x1.Re;
		x1.Im = x0.Im - x1.Im;
		x0.Re += x1W.Re;
		x0.Im += x1W.Im;
	}

	// special 4-point FFT for the second pass
	template <class BuffType>
	inline void FFT4(BuffType &x0, BuffType &x1, BuffType &x2, BuffType &x3) {
		Type x1W;
		x1W.Re = x2.Re;
		x1W.Im = x2.Im;
		x2.Re = x0.Re - x1W.Re;
		x2.Im = x0.Im - x1W.Im;
		x0.Re = x0.Re + x1W.Re;
		x0.Im = x0.Im + x1W.Im;
		x1W.Re = x3.Im;
		x1W.Im = (-x3.Re);   
		x3.Re = x1.Re - x1W.Re;
		x3.Im = x1.Im - x1W.Im;
		x1.Re = x1.Re + x1W.Re;
		x1.Im = x1.Im + x1W.Im;
	}

};


#endif // __FFT_H__

