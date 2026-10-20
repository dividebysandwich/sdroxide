// MFSK trasnmitter and receiver code, Pawel Jalocha, December 2004

#ifndef __PJ_MFSK_H__
#define __PJ_MFSK_H__

// =====================================================================

#include <stdlib.h>
#include <stdint.h>
#include <math.h>

#include "pj_struc.h"
#include "pj_fht.h"
#include "pj_cmpx.h"
#include "pj_fft.h"
#include "pj_gray.h"
#include "pj_lowpass3.h"
#include "pj_fifo.h"

// =====================================================================

static inline size_t Exp2(uint32_t X) { 
	return (uint32_t)1 << X; 
}

static inline size_t Log2(uint32_t X) {
	uint32_t Y;
	for ( Y = 0; X > 1; X >>= 1)
	Y += 1;
	return Y; 
}

// =====================================================================

// the symbol shape described in frequency domain
static const double MFSK_SymbolFreqShape[] =
 { 1.0, 1.0 } ; // use raised cosine shape - experimental
// from gMFSK
// {	+1.0000000000, 
//	+1.1913785723, 
//	-0.0793018558, 
//	-0.2171442026, 
//	-0.0014526076 
//};
// from DM780
//{
//	+1.0000000000,
//	+2.1373197349,
//	+1.1207588117,
//	-0.0165609232 
//};

static const size_t MFSK_SymbolFreqShapeLen = 
	sizeof(MFSK_SymbolFreqShape) / sizeof(double);

// =====================================================================

template <class Type=float>
class MFSK_Modulator
{
public:
// parameters to be set before calling Preset()
	size_t SymbolLen;		// length of the symbol, must be a power of 2
	size_t FirstCarrier;	// first carrier in terms of FFT freq. bins
	size_t BitsPerSymbol;	// bits per symbol => number of carriers/tones
	int UseGrayCode;

	static const size_t CarrierSepar = 2;

// parameters that are calculated by Preset()
	size_t Carriers;		// number of tones/carriers
	size_t SymbolSepar;	// time distance between symbols
	int	SymbolPhase;		// the phase of the tone being transmitted
	int	Reverse;			// send carriers in reverse order?

private:
	Type *CosineTable;		// Cosine table for fast cos/sin calculation
	Type *SymbolShape;		// the shape of the symbol
	Type	*OutTap;			// output tap (buffer)
	size_t TapPtr;
	size_t WrapMask;

public:

	MFSK_Modulator() {
		Init();
		Default();
	}

	~MFSK_Modulator() {
		Free();
	}

	void Init(void) {
		CosineTable = 0;
		SymbolShape = 0;
		OutTap = 0;
	}

	void Free(void) {
		delete [] CosineTable; 
		CosineTable = 0;
		delete [] SymbolShape;
		SymbolShape = 0;
		delete [] OutTap;
		OutTap = 0;
	}

	void Default(void) {
		SymbolLen = 512;
		FirstCarrier = 32;
		BitsPerSymbol = 5;
		Reverse = 0;
		UseGrayCode = 1;
	}

	int Preset(void) {
		size_t Idx;

		Carriers = Exp2(BitsPerSymbol);
		SymbolSepar = SymbolLen / 2;

		if (ReallocArray(&CosineTable, SymbolLen) < 0) goto Error;
		for (Idx = 0; Idx < SymbolLen; Idx++)
			CosineTable[Idx] = cos((2*M_PI*Idx) / SymbolLen);

		if (ReallocArray(&SymbolShape, SymbolLen) < 0) goto Error;

		{	size_t Time;
			double Ampl = MFSK_SymbolFreqShape[0];
			for (Time = 0; Time < SymbolLen; Time++)
				SymbolShape[Time] = Ampl;
		}
		size_t Freq;
		for (Freq = 1; Freq < MFSK_SymbolFreqShapeLen; Freq++) {
			size_t Time;
			double Ampl = MFSK_SymbolFreqShape[Freq];
			if (Freq & 1) Ampl = (-Ampl);
			size_t Phase = 0;
			for (Time = 0; Time < SymbolLen; Time++) {
				SymbolShape[Time] += Ampl*CosineTable[Phase];
				Phase += Freq; 
				if (Phase >= SymbolLen) Phase-=SymbolLen;
			}
		}
		{	size_t Time;
			double Scale = 1.0/4;
			for (Time = 0; Time < SymbolLen; Time++)
			SymbolShape[Time] *= Scale;
		}

		if (ReallocArray(&OutTap, SymbolLen) < 0) goto Error;
		for (Idx = 0; Idx < SymbolLen; Idx++)
			OutTap[Idx] = 0;
		TapPtr = 0;

		WrapMask = SymbolLen-1;
		SymbolPhase = 0;

		return 0;

	Error:
		Free();
		return -1;
	}

	void Send(uint8_t Symbol) {
		if (UseGrayCode) Symbol=GrayCode(Symbol);
		int SymbolFreq;

		if (Reverse == 1) { int RevFirstCar=FirstCarrier-2; SymbolFreq=RevFirstCar-CarrierSepar*Symbol; }
		else { SymbolFreq=FirstCarrier+CarrierSepar*Symbol; }

		int TimeShift=SymbolSepar/2-SymbolLen/2;
		SymbolPhase+=SymbolFreq*TimeShift;
		SymbolPhase&=WrapMask;

		AddSymbol(SymbolFreq,SymbolPhase);

		TimeShift=SymbolSepar/2+SymbolLen/2;
		SymbolPhase+=SymbolFreq*TimeShift;
		SymbolPhase&=WrapMask;

		int PhaseDiffer=SymbolLen/4;
		if (rand()&1) PhaseDiffer=(-PhaseDiffer);
		SymbolPhase+=PhaseDiffer;
		SymbolPhase&=WrapMask;
		}
		
// get output as 16-bit signed data
	int Output(int16_t *Buffer) {
		const Type Scale = 32768.0;
		const int32_t Limit = 0x7FFF;
		size_t Idx;

		for (Idx = 0; Idx < SymbolSepar; Idx++) {
			Type Ampl = OutTap[TapPtr];
			Ampl *= Scale;
			int32_t Out = (int32_t)floor(Ampl + 0.5);
			if (Out > Limit) Out = Limit;
			else if (Out < (-Limit)) Out = (-Limit);
			Buffer[Idx] = (int16_t)Out;
			OutTap[TapPtr] = 0;
			TapPtr += 1; TapPtr &= WrapMask;
		}
		return SymbolSepar;
	}

	template <class OutType>
	int Output(OutType *Buffer) {
		size_t Idx;
		for (Idx = 0; Idx < SymbolSepar; Idx++) {
			Buffer[Idx] = OutTap[TapPtr];
			OutTap[TapPtr] = 0;
			TapPtr += 1;
			TapPtr &= WrapMask;
		}
		return SymbolSepar;
	}

private:

	void AddSymbol(int Freq, int Phase) {
		size_t Time;
		for (Time = 0; Time < SymbolLen; Time++) {
// experimental use with {1.0, 1.0};
			Type Shape=1.0-CosineTable[Time];
			OutTap[TapPtr] += CosineTable[Phase] * Shape;
//				OutTap[TapPtr] += CosineTable[Phase] * SymbolShape[Time];

			Phase += Freq;
			Phase &= WrapMask;
			TapPtr += 1;
			TapPtr &= WrapMask;
		}
	}

} ;

// =====================================================================

template <class TapType = float, class OutType = double>
class BoxFilter
{
public:
	size_t	Len;
	TapType *Tap;
	size_t	Ptr;
	OutType Output;

	BoxFilter() {
			Tap = 0;
	}

	~BoxFilter() {
			delete [] Tap;
	}

	void Free(void) {
			delete [] Tap;
			Tap = 0;
	}

	int Preset(void) {
			if (ReallocArray(&Tap, Len) < 0) 
				return -1;
			Clear();
			return 0;
	}

	void Clear(void) {
			size_t Idx;
			for (Idx = 0; Idx < Len; Idx++)
				Tap[Idx] = 0;
			Ptr = 0;
			Output = 0;
	}

	template <class InpType>
	void Process(InpType Input) {
			Output -= Tap[Ptr];
			Output += Input;
			Tap[Ptr] = Input;
			Ptr += 1;
			if (Ptr >= Len) Ptr -= Len;
	}

};

// =====================================================================

template <class Type>
class MFSK_InputProcessor
{
public:
	size_t	WindowLen;
	size_t	WrapMask;
	Type	*InpTap;
	size_t	InpTapPtr;
	Type	*OutTap;
	size_t	OutTapPtr;
	Type	*WindowShape;
	size_t	SliceSepar;

	r2FFT< Cmpx<Type> > FFT;				// FFT engine
	Cmpx<Type> *FFT_Buff;					// FFT buffer

	size_t	SpectraLen;
	Cmpx<Type> *Spectra[2];

	Type *Output;
	Type *Energy;

	BoxFilter<Type> Filter;

public:
	MFSK_InputProcessor() {
			Init();
			Default();
	}

	~MFSK_InputProcessor() {
			Free();
	}
	
	void Init(void) {
			InpTap = 0;
			OutTap = 0;
			WindowShape = 0;
			FFT_Buff = 0;
			Spectra[0] = 0;
			Spectra[1] = 0;
			Output = 0;
			Energy = 0;
	}

	void Free(void) {
			delete [] InpTap; InpTap = 0;
			delete [] OutTap; OutTap = 0;
			delete [] WindowShape; WindowShape = 0;
			delete [] FFT_Buff; FFT_Buff = 0;
			delete [] Spectra[0]; Spectra[0] = 0;
			delete [] Spectra[1]; Spectra[1] = 0;
			delete [] Output; Output = 0;
			delete [] Energy; Energy = 0;
			FFT.Free();
			Filter.Free();
	}

	void Default(void) {
			WindowLen = 8192;
	}
		
	int Preset(void) {
			size_t Idx;
			WrapMask = WindowLen - 1;
			Type ShapeScale = 2.0 / WindowLen;

			if (ReallocArray(&InpTap, WindowLen) < 0) goto Error;
			ClearArray(InpTap, WindowLen);
			InpTapPtr = 0;
			if (ReallocArray(&OutTap, WindowLen) < 0) goto Error;
			ClearArray(OutTap, WindowLen);
			OutTapPtr = 0;

			if (FFT.Preset(WindowLen) < 0) goto Error;
			if (ReallocArray(&FFT_Buff, WindowLen) < 0) goto Error;
			SliceSepar = WindowLen / 2;

			if (ReallocArray(&WindowShape, WindowLen)< 0) goto Error;
			for (Idx = 0; Idx < WindowLen; Idx++)
				WindowShape[Idx] = ShapeScale * sqrt(1.0 - FFT.Twiddle[Idx].Re);

			SpectraLen = WindowLen / 2;
			if (ReallocArray(&Spectra[0], SpectraLen) < 0) goto Error;
			if (ReallocArray(&Spectra[1], SpectraLen) < 0) goto Error;

			if (ReallocArray(&Output, WindowLen) < 0) goto Error;
			ClearArray(Output, WindowLen);

			if (ReallocArray(&Energy, SpectraLen) < 0) goto Error;

			Filter.Len = WindowLen / 16;
			if (Filter.Preset() < 0) goto Error;

			return 0;
		
	Error:
			Free();
			return -1;
	}

	void Reset(void) {
			ClearArray(InpTap, WindowLen);
			InpTapPtr = 0;
			ClearArray(OutTap, WindowLen);
			OutTapPtr = 0;
	}

	void LimitSpectraPeaks( Cmpx<Type> *Spectra,
									size_t Len = 64, 
									Type Threshold = 4.0) {
			Filter.Len = Len;
			Filter.Preset();

			size_t MaxFreq = 3 * (SpectraLen / 4);
			size_t Freq, Idx;

			for (Freq = 0; Freq < Len; Freq++)
				Filter.Process(Energy[Freq]);

			for (Idx = Len / 2; Freq < MaxFreq; Freq++,Idx++) {
				Filter.Process(Energy[Freq]);
				Type Signal = Energy[Idx];
				Type Limit = (Filter.Output/Len) * Threshold;
				if (Signal > Limit) {
					Spectra[Idx] *= sqrt(Limit / Signal);
					Energy[Idx] = Limit;
				}
			}
	}

	void LimitOutputPeaks(Type Threshold = 2.5) {
			size_t Idx;
			Type RMS = 0;
			for (Idx = 0; Idx < WindowLen; Idx++) {
				Type Signal = Output[Idx];
				RMS += Signal * Signal;
			}
			RMS = sqrt(RMS / WindowLen);
			Type Limit = RMS * Threshold;

			for (Idx = 0; Idx < WindowLen; Idx++) {
				Type Signal = Output[Idx];
				if (Signal > Limit)
					Output[Idx] = Limit;
				else if (Signal < (-Limit))
					Output[Idx] = (-Limit);
			}
	}

	void AverageEnergy(size_t Len = 32) {
			Filter.Len = Len;
			Filter.Preset();

			size_t MaxFreq = 3 * (SpectraLen / 4);
			Type Scale = 1.0 / Len;
			size_t Len2 = Len / 2;
			size_t Idx, Freq;

			for (Freq = 0; Freq < Len; Freq++)
				Filter.Process(Energy[Freq]);

			for (Idx = 0; Idx < Len2; Idx++)
				Energy[Idx] = Filter.Output * Scale;

			for ( ; Freq < MaxFreq; Freq++,Idx++) {
				Filter.Process(Energy[Freq]);
				Energy[Idx] = Filter.Output * Scale;
			}

			for ( ; Idx < SpectraLen; Idx++)
				Energy[Idx] = Filter.Output*Scale;
	}

// here we process the spectral data
	void ProcessSpectra(Cmpx<Type> *Spectra) {
			size_t Freq;
			for (Freq = 0; Freq < SpectraLen; Freq++)
				Energy[Freq] = Spectra[Freq].Energy();

			LimitSpectraPeaks(Spectra, WindowLen / 64, 4.0);
			LimitSpectraPeaks(Spectra, WindowLen / 64, 4.0);
			LimitSpectraPeaks(Spectra, WindowLen / 64, 4.0);

			AverageEnergy(WindowLen / 96);
			AverageEnergy(WindowLen / 64);

			for (Freq = 0; Freq < SpectraLen; Freq++) {
				Type Corr = Energy[Freq];
				if (Corr <= 0) continue;
				Corr = 1.0 / sqrt(Corr);
				Spectra[Freq] *= Corr;
			}
	}

	template <class InpType>
	void ProcessInpTap(InpType *Input) {
			size_t InpIdx;
			for (InpIdx = 0; InpIdx < SliceSepar; InpIdx++) {
				InpTap[InpTapPtr] = Input[InpIdx];
				InpTapPtr += 1;
				InpTapPtr &= WrapMask;
			}
	}

	void ProcessInpTap() {
			size_t InpIdx;
			for (InpIdx = 0; InpIdx < SliceSepar; InpIdx++) {
				InpTap[InpTapPtr] = 0;
				InpTapPtr += 1;
				InpTapPtr &= WrapMask;
			}
	}

	void ProcessInpWindow_Re(void) {
			size_t Time;
			for (Time = 0; Time < WindowLen; Time++) {
				FFT_Buff[Time].Re = InpTap[InpTapPtr] * WindowShape[Time];
				InpTapPtr += 1;
				InpTapPtr &= WrapMask;
			}
	}

	void ProcessInpWindow_Im(void) {
			size_t Time;
			for (Time = 0; Time < WindowLen; Time++) {
				FFT_Buff[Time].Im = InpTap[InpTapPtr] * WindowShape[Time];
				InpTapPtr += 1;
				InpTapPtr &= WrapMask;
			}
	}

	void ProcessOutWindow_Re(void)
		{ size_t Time;
		for (Time=0; Time<WindowLen; Time++)
		{ OutTap[OutTapPtr] += FFT_Buff[Time].Re*WindowShape[Time];
			OutTapPtr+=1; OutTapPtr&=WrapMask; }
		}

	void ProcessOutWindow_Im(void) {
			size_t Time;
			for (Time = 0; Time < WindowLen; Time++) {
				OutTap[OutTapPtr] += FFT_Buff[Time].Im * WindowShape[Time];
				OutTapPtr += 1;
				OutTapPtr &= WrapMask;
			}
	}

	void ProcessOutTap(Type *Output) {
			size_t OutIdx;
			for (OutIdx = 0; OutIdx < SliceSepar; OutIdx++) {
				Output[OutIdx] = OutTap[OutTapPtr];
				OutTap[OutTapPtr] = 0;
				OutTapPtr += 1;
				OutTapPtr &= WrapMask;
			}
	}

	template <class InpType>
	int Process(InpType *Input) {

			if (Input) ProcessInpTap(Input);
			else ProcessInpTap();
			ProcessInpWindow_Re();

			if (Input) ProcessInpTap(Input+SliceSepar);
			else ProcessInpTap();
			ProcessInpWindow_Im();

			FFT.Process(FFT_Buff);
			FFT.SeparTwoReals(FFT_Buff, Spectra[0], Spectra[1]);

			ProcessSpectra(Spectra[0]);
			ProcessSpectra(Spectra[1]);

			FFT.JoinTwoReals(Spectra[0], Spectra[1], FFT_Buff);
			FFT.Process(FFT_Buff);

			ProcessOutWindow_Re();
			ProcessOutTap(Output);
			ProcessOutWindow_Im();
			ProcessOutTap(Output+SliceSepar);

			LimitOutputPeaks(2.5);
			LimitOutputPeaks(2.5);

			return WindowLen;
	}

// get output as 16-bit signed data
	int GetOutput(int16_t *Buffer) {
			const Type Scale = 32768.0;
			const int32_t Limit = 0x7FFF;
			size_t Idx;

			for (Idx = 0; Idx < WindowLen; Idx++) {
				Type Ampl = Output[Idx];
				Ampl *= Scale;
				int32_t Out = (int32_t)floor(Ampl + 0.5);
				if (Out > Limit) Out = Limit;
				else if (Out < (-Limit)) Out = (-Limit);
				Buffer[Idx] = (int16_t)Out;
			}
			return WindowLen;
	}

} ;

// =====================================================================

// A circular buffer to store history of data.
// Data may come as single numbers or in batches
// of fixed size (-> Width)
template <class Type>
class CircularBuffer
{ 
public:
	size_t Width;	// input/output data width (row width)
	size_t Len;	// buffer length (column height)
	size_t Size;	// total size of the storage in the buffer
	size_t Ptr;	// current pointer (counts rows)
	Type	*Data;	// allocated storage

	CircularBuffer() { Init(); }
	~CircularBuffer() { delete [] Data; }

	void Init(void)
	{ Data = 0; Size = 0; Width = 1; }

	void Free(void)
	{ delete [] Data; Data = 0; Size = 0; }

// reset: set pointer to the beginning of the buffer
	void Reset(void) { Ptr = 0; }

// preset for given length and width
	int Preset(void) { 
			Size = Width * Len;
			if (ReallocArray(&Data, Size) < 0) return -1;
			Reset(); 
			return 0;
	}

// set all elements to given value
	void Set(Type &Value) {
			size_t Idx;
			for (Idx = 0; Idx < Size; Idx++)
			Data[Idx]=Value;
	}

// set all elements to zero
	void Clear(void) {
			Type Zero; 
			Zero = 0; 
			Set(Zero); 
	}

// increment the pointer (with wrapping around)
	void IncrPtr(size_t &Ptr, size_t Step=1) {
			Ptr += Step; 
			if (Ptr >= Len) Ptr -= Len;
	}

// decrement the pointer (with wrapping around)
	void DecrPtr(size_t &Ptr, size_t Step=1) {
			if (Ptr >= Step) Ptr -= Step;
			else				Ptr += (Len - Step);
	}

// synchronize current pointer with another circular buffer
	template <class SrcType>
	void operator |= (CircularBuffer<SrcType> Buffer) { 
			Ptr = Buffer.Ptr; 
	}

// advance (increment) current pointer
	void operator += (size_t Step) { 
			IncrPtr(Ptr, Step); 
	}

// decrement current pointer
	void operator -= (size_t Step) { 
			DecrPtr(Ptr, Step);
	}

// index operator to get the absolute data pointer
	Type *operator [] (size_t Idx) {
			return Data + (Idx * Width);
	}

// get storage pointer corresponding to an absolute pipe pointer
	Type *AbsPtr(size_t Ptr) {
			return Data + (Ptr * Width);
	}

// get storage pointer corresponding to current pipe pointer
	Type *CurrPtr(void) {
			return Data + (Ptr * Width);
	}

// get storage pointer corresponding to current pointer +/- offset
	Type *OffsetPtr(int Offset) {
			Offset += Ptr; 
			Offset *= Width;
			if (Offset < 0) Offset += Size;
			else if (Offset >= (int)Size) Offset -= Size;
			return Data + Offset;
	}
};

// =====================================================================

template <class Type=float>
 class MFSK_Demodulator
{ public:

	// parameters to be set before calling Preset()
	size_t SymbolLen;		// length of the symbol, must be a power of 2
	size_t FirstCarrier;	// first carrier in terms of FFT freq. bins
	size_t BitsPerSymbol;	// bits per symbol => number of carriers/tones
	int UseGrayCode;
	size_t DecodeMargin;
	int EqualizerDepth;	// leave this at 0 (disable the equalizer)
	int Reverse;

	static const size_t CarrierSepar=2;
	static const size_t SpectraPerSymbol = 2; // FFT slices per symbol

	public:

	size_t Carriers;		// number of tones/carriers
	size_t SymbolSepar;	// time distance between symbols

	private:

	size_t DecodeWidth;

	size_t SymbolSepar2;

	size_t WrapMask;

	Type *InpTap;								// input buffer
	size_t InpTapPtr;

	Type *SymbolShape;						// the shape of the symbol and the FFT window

	r2FFT< Cmpx<Type> > FFT;				// FFT engine
	Cmpx<Type> *FFT_Buff;					// FFT buffer

	size_t	SpectraLen;							// number of spectra points per FFT
	Cmpx<Type> *Spectra[SpectraPerSymbol]; // two buffers for FFT spectra
	Type		*Energy[SpectraPerSymbol];

	CircularBuffer<Type> EnergyBuffer;
	LowPass3_Filter<Type> *AverageEnergy;
	Type FilterWeight;

	public:

	MFSK_Demodulator()
	{ Init();
		Default(); }

	~MFSK_Demodulator()
	{ Free(); }
	
	void Init(void)
		{ InpTap=0;
		SymbolShape=0;
		FFT_Buff=0;
		Spectra[0]=0;
		Spectra[1]=0;
		Energy[0]=0;
		Energy[1]=0;
		AverageEnergy=0; }

	void Free(void) {
		delete [] InpTap; InpTap=0;
		delete [] SymbolShape; SymbolShape=0;
		delete [] FFT_Buff; FFT_Buff=0;
		delete [] Spectra[0]; Spectra[0]=0;
		delete [] Spectra[1]; Spectra[1]=0;
		delete [] Energy[0]; Energy[0]=0;
		delete [] Energy[1]; Energy[1]=0;
		delete [] AverageEnergy; AverageEnergy=0;
		FFT.Free();
		EnergyBuffer.Free(); }

	void Default(void)
		{ SymbolLen=512;
		FirstCarrier=32;
		BitsPerSymbol=5;
		UseGrayCode=1;
		DecodeMargin=32;
		Reverse=0;
		EqualizerDepth=0; }

	int Preset(void)
		{

		Carriers=Exp2(BitsPerSymbol);

		WrapMask=SymbolLen-1;

		Type ShapeScale=1.0/SymbolLen;

		if (ReallocArray(&InpTap,SymbolLen)<0) goto Error;
		ClearArray(InpTap,SymbolLen);
		InpTapPtr=0;

		if (FFT.Preset(SymbolLen)<0) goto Error;
		if (ReallocArray(&FFT_Buff,SymbolLen)<0) goto Error;
		SymbolSepar=SymbolLen/2;
		SymbolSepar2=SymbolSepar/2;

		if (ReallocArray(&SymbolShape,SymbolLen)<0) goto Error;

		{ size_t Time;
			double Ampl=MFSK_SymbolFreqShape[0];
			for (Time=0; Time<SymbolLen; Time++)
				SymbolShape[Time]=Ampl;
		}
		size_t Freq;
		for (Freq=1; Freq<MFSK_SymbolFreqShapeLen; Freq++)
		{ size_t Time;
			double Ampl=MFSK_SymbolFreqShape[Freq];
			if (Freq&1) Ampl=(-Ampl);
			size_t Phase=0;
			for (Time=0; Time<SymbolLen; Time++)
			{ SymbolShape[Time]+=Ampl*FFT.Twiddle[Phase].Re;
				Phase+=Freq; if (Phase>=SymbolLen) Phase-=SymbolLen; }
		}
		{ size_t Time;
			for (Time=0; Time<SymbolLen; Time++)
				SymbolShape[Time]*=ShapeScale;
		}

		SpectraLen=SymbolLen/2;
		if (ReallocArray(&Spectra[0],SpectraLen)<0) goto Error;
		if (ReallocArray(&Spectra[1],SpectraLen)<0) goto Error;

		if (DecodeMargin>FirstCarrier) DecodeMargin=FirstCarrier;
		DecodeWidth=(Carriers*CarrierSepar-1)+2*DecodeMargin;

		if (ReallocArray(&Energy[0],DecodeWidth)<0) goto Error;
		if (ReallocArray(&Energy[1],DecodeWidth)<0) goto Error;

		if (EqualizerDepth)
		{ EnergyBuffer.Len=EqualizerDepth;
			EnergyBuffer.Width=DecodeWidth;
			if (EnergyBuffer.Preset()<0) goto Error;
			EnergyBuffer.Clear();
			if (ReallocArray(&AverageEnergy,DecodeWidth)<0) goto Error;
			size_t Idx;
			for (Idx=0; Idx<DecodeWidth; Idx++)
				AverageEnergy[Idx]=0;
			FilterWeight=1.0/EqualizerDepth;
		}
		else
		{ EnergyBuffer.Free();
			if (AverageEnergy)
			{ delete [] AverageEnergy; AverageEnergy=0; }
		}

		return 0;
		
		Error: Free(); return -1; }

	template <class InpType>
	void Process(InpType *Input)
		{ size_t InpIdx,Time;

		for (InpIdx=0; InpIdx<SymbolSepar2; InpIdx++)
		{ InpTap[InpTapPtr]=Input[InpIdx];
			InpTapPtr+=1; InpTapPtr&=WrapMask; }

		for (Time=0; Time<SymbolLen; Time++)
		{ FFT_Buff[Time].Re=InpTap[InpTapPtr]*SymbolShape[Time];
			InpTapPtr+=1; InpTapPtr&=WrapMask; }

		for (			; InpIdx<SymbolSepar ; InpIdx++)
		{ InpTap[InpTapPtr]=Input[InpIdx];
			InpTapPtr+=1; InpTapPtr&=WrapMask; }

		for (Time=0; Time<SymbolLen; Time++)
		{ FFT_Buff[Time].Im=InpTap[InpTapPtr]*SymbolShape[Time];
			InpTapPtr+=1; InpTapPtr&=WrapMask; }

		FFT.Process(FFT_Buff);
		FFT.SeparTwoReals(FFT_Buff, Spectra[0], Spectra[1]);

		if (EqualizerDepth)
		{ size_t Idx,Freq;
			Type *Data0 = EnergyBuffer.OffsetPtr(0);
			Type *Data1 = EnergyBuffer.OffsetPtr(1);

			for (Idx=0; Idx<DecodeWidth; Idx++)
			{ Energy[0][Idx]=Data0[Idx];
				Energy[1][Idx]=Data1[Idx]; }

			if (Reverse==1) {
			Freq=FirstCarrier;
			for (Idx=0; Idx<DecodeWidth; Idx++, Freq--)
			{ Type Energy0=Spectra[0][Freq].Energy();
				Data0[Idx]=Energy0;
				AverageEnergy[Idx].Process(Energy0,FilterWeight);
				Type Energy1=Spectra[1][Freq].Energy();
				Data1[Idx]=Energy1;
				AverageEnergy[Idx].Process(Energy1,FilterWeight); }
			} else {
			Freq=FirstCarrier-DecodeMargin;
			for (Idx=0; Idx<DecodeWidth; Idx++, Freq++)
			{ Type Energy0=Spectra[0][Freq].Energy();
				Data0[Idx]=Energy0;
				AverageEnergy[Idx].Process(Energy0,FilterWeight);
				Type Energy1=Spectra[1][Freq].Energy();
				Data1[Idx]=Energy1;
				AverageEnergy[Idx].Process(Energy1,FilterWeight); }
			}
/*
			for (Idx=0; Idx<DecodeWidth; Idx++, Freq++)
			{ Type RefEnergy=AverageEnergy[Idx].Output;
				if (RefEnergy>0)
				{ Energy[0][Idx]/=RefEnergy;
				Energy[1][Idx]/=RefEnergy; }
				else
				{ Energy[0][Idx]=0;
				Energy[1][Idx]=0; }
			}
*/			
			EnergyBuffer+=2;
		}
		else
		{ size_t Idx;
			if (Reverse==1) {
			size_t Freq=FirstCarrier;
			for (Idx=0; Idx<DecodeWidth; Idx++, Freq--)
			{ Energy[0][Idx]=Spectra[0][Freq].Energy(); 
				Energy[1][Idx]=Spectra[1][Freq].Energy(); }
			} else {
			size_t Freq=FirstCarrier-DecodeMargin;
			for (Idx=0; Idx<DecodeWidth; Idx++, Freq++)
			{ Energy[0][Idx]=Spectra[0][Freq].Energy();
				Energy[1][Idx]=Spectra[1][Freq].Energy(); }
			}
		}
		}

		uint8_t HardDecode(size_t Slice=0, int FreqOffset=0)
		{ size_t Idx;
		Type Peak=0;
		uint8_t PeakIdx=0;
		Type *EnergyPtr=Energy[Slice]+(DecodeMargin+FreqOffset);
		size_t Freq=0;
		for (Idx=0; Idx<Carriers; Idx++)
		{ Type Energy=EnergyPtr[Freq];
			if (Energy>Peak)
			{ Peak=Energy; PeakIdx=Idx; }
			Freq+=CarrierSepar; }

		if (UseGrayCode) PeakIdx=BinaryCode(PeakIdx);
		return PeakIdx; }

		template <class SymbType>
		void SoftDecode(SymbType *Symbol,
						size_t Slice=0, int FreqOffset=0)
		{ size_t Bit,Idx;
		for (Bit=0; Bit<BitsPerSymbol; Bit++)
			Symbol[Bit]=0;

		Type *EnergyPtr=Energy[Slice]+(DecodeMargin+FreqOffset);
		Type TotalEnergy=0;
		size_t Freq=0;
		for (Idx=0; Idx<Carriers; Idx++)
		{ uint8_t SymbIdx=Idx;
			if (UseGrayCode) SymbIdx=BinaryCode(SymbIdx);
			Type Energy=EnergyPtr[Freq];
			Energy*=Energy;
			TotalEnergy+=Energy;
			uint8_t Mask=1;
			for (Bit=0; Bit<BitsPerSymbol; Bit++)
			{ if (SymbIdx&Mask) Symbol[Bit]-=Energy;
								else Symbol[Bit]+=Energy;
				Mask<<=1; }
			Freq+=CarrierSepar; }

		if (TotalEnergy>0)
		{ for (Bit=0; Bit<BitsPerSymbol; Bit++)
				Symbol[Bit]/=TotalEnergy; }

		}

		template <class SymbType>
		void SoftDecode_Test(SymbType *Symbol,
						size_t Slice=0, int FreqOffset=0)
		{ size_t Bit,Idx,Freq;

		Type *EnergyPtr=Energy[Slice]+(DecodeMargin+FreqOffset);

//		printf("SoftDecode:");

		Type TotalEnergy=0;
		Type PeakEnergy=0;
		for (Freq=0,Idx=0; Idx<Carriers; Idx++,Freq+=CarrierSepar)
		{ Type Energy=EnergyPtr[Freq];
			TotalEnergy+=Energy;
			if (Energy>PeakEnergy) PeakEnergy=Energy; }
		Type AverageNoise=(TotalEnergy-PeakEnergy)/(Carriers-1)/2;
/*
		printf("	PeakEnergy/TotalEnergy=%4.3f",PeakEnergy/TotalEnergy);
		printf("	AverageNoise/TotalEnergy=%4.3f\n",AverageNoise/TotalEnergy);
		printf("Energy[%d]/Tot =",Carriers);
		for (Freq=0,Idx=0; Idx<Carriers; Idx++,Freq+=CarrierSepar)
			printf(" %4.3f",EnergyPtr[Freq]/TotalEnergy);
		printf("\n");
*/
		Type SymbolProb[Carriers];

		for (Freq=0,Idx=0; Idx<Carriers; Idx++,Freq+=CarrierSepar)
		{ Type Energy=EnergyPtr[Freq];
			Type NoiseEnergy=TotalEnergy-Energy;
			SymbolProb[Idx]=exp(-NoiseEnergy/(2*AverageNoise)); }
/*
		printf("SymbolProb[%d] =",Carriers);
		for (Idx=0; Idx<Carriers; Idx++)
			printf(" %4.3f",SymbolProb[Idx]);
		printf("\n");
*/
		Type ProbCorr=0;
		for (Idx=0; Idx<Carriers; Idx++)
			ProbCorr+=SymbolProb[Idx];
		ProbCorr=1.0/ProbCorr;
		for (Idx=0; Idx<Carriers; Idx++)
			SymbolProb[Idx]*=ProbCorr;
/*
		printf("SymbolProb[%d] =",Carriers);
		for (Idx=0; Idx<Carriers; Idx++)
			printf(" %4.3f",SymbolProb[Idx]);
		printf("\n");
*/
		uint8_t Mask=1;
		for (Bit=0; Bit<BitsPerSymbol; Bit++)
		{ Type Prob0=0;
			Type Prob1=0;
			for (Idx=0; Idx<Carriers; Idx++)
			{ uint8_t SymbIdx=Idx;
				if (UseGrayCode) SymbIdx=BinaryCode(SymbIdx);
				if (SymbIdx&Mask) Prob1+=SymbolProb[Idx];
								else Prob0+=SymbolProb[Idx];
			}
			Symbol[Bit]=log(Prob0/Prob1);
			Mask<<=1; }
		}

} ;

// =====================================================================

template <class Type>
 void PrintBinary(Type Number, size_t Bits)
{ Type Mask=1; Mask<<=(Bits-1);
	for ( ; Bits; Bits--)
	{ printf("%c",Number&Mask ? '1':'0');
	Mask>>=1; }
}

// =====================================================================

class MFSK_Encoder
{ public:

	size_t BitsPerSymbol;
	size_t BitsPerCharacter;

	public:

	size_t Symbols;
	size_t SymbolsPerBlock;

	bool	bContestia;
//	bool	bRTTYM;

	private:

   static const uint64_t ScramblingCodeOlivia = 0xE257E6D0291574ECLL;
   static const uint64_t ScramblingCodeContestia = 0xEDB88320LL;
//   static const uint64_t ScramblingCodeRTTYM = 0xEDB88320LL;
   uint64_t ScramblingCode;

	int8_t *FHT_Buffer;

	public:

	uint8_t *OutputBlock;

	public:

	MFSK_Encoder() { 
		Default();
		Init();
	}

	~MFSK_Encoder()
		{ Free(); }

	void Default(void) { 
		bContestia = false;
		BitsPerSymbol=5;
		BitsPerCharacter=7; 
		ScramblingCode = ScramblingCodeOlivia;
	}

	void Init(void)
		{ FHT_Buffer=0;
		OutputBlock=0; }

	void Free(void)
		{ delete [] FHT_Buffer; FHT_Buffer=0;
		delete [] OutputBlock; OutputBlock=0; }

	int Preset(void) { 
		if (bContestia) {
			ScramblingCode = ScramblingCodeContestia;
			BitsPerCharacter = 6;
//		} else if (bRTTYM) {
//			ScramblingCode = ScramblingCodeRTTYM;
//			BitsPerCharacter = 5;
		} else { // standard Olivia
				ScramblingCode = ScramblingCodeOlivia;
				BitsPerCharacter = 7;
		}
		Symbols = 1 << BitsPerSymbol;
		SymbolsPerBlock = Exp2(BitsPerCharacter-1);
		if (ReallocArray(&FHT_Buffer,SymbolsPerBlock)<0) goto Error;
		if (ReallocArray(&OutputBlock,SymbolsPerBlock)<0) goto Error;
		return 0;
		Error: 
			Free(); 
			return -1;
		}

	void EncodeCharacter(uint8_t Char) {
		size_t TimeBit;
		uint8_t Mask = (SymbolsPerBlock << 1) - 1;

		if (bContestia) {
			if (Char >= 'a' && Char <= 'z')
				Char += 'A' - 'a';
			if (Char == ' ')
				Char = 59;
			else if (Char == '\r')
				Char = 60;
			else if (Char == '\n')
				Char = 0;
			else if (Char >= 33 && Char <= 90)
				Char -= 32;
			else if (Char == 8)
				Char = 61;
			else if (Char == 0)
				Char = 0;
			else
				Char = '?' - 32;
//		} else if (bRTTYM) {
		} else {
			Char &= Mask;
		}
		
		for (TimeBit = 0; TimeBit < SymbolsPerBlock; TimeBit++)
			FHT_Buffer[TimeBit] = 0;
		if (Char<SymbolsPerBlock) 
			FHT_Buffer[Char] = 1;
		else
			FHT_Buffer[Char-SymbolsPerBlock] = (-1);
		IFHT(FHT_Buffer, SymbolsPerBlock);
		}

	void ScrambleFHT(size_t CodeOffset=0)
		{ size_t TimeBit;
		size_t CodeWrap=(SymbolsPerBlock-1);
		size_t CodeBit=CodeOffset&CodeWrap;
		for (TimeBit=0; TimeBit<SymbolsPerBlock; TimeBit++)
		{ uint64_t CodeMask=1; CodeMask<<=CodeBit;
			if (ScramblingCode&CodeMask)
				FHT_Buffer[TimeBit] = (-FHT_Buffer[TimeBit]);
		CodeBit+=1; CodeBit&=CodeWrap; }
		}

	void EncodeBlock(uint8_t *InputBlock) {
		size_t FreqBit;
		size_t TimeBit;
		size_t nShift;

//		nShift = (bContestia || bRTTYM) ? 5 : 13; // Contestia/RTTYM or Olivia
		nShift = (bContestia) ? 5 : 13; // Contestia/RTTYM or Olivia

		for (TimeBit = 0; TimeBit < SymbolsPerBlock; TimeBit ++)
			OutputBlock[TimeBit] = 0;

		for (FreqBit = 0; FreqBit < BitsPerSymbol; FreqBit++) {
			EncodeCharacter(InputBlock[FreqBit]);
			ScrambleFHT(FreqBit * nShift);
			size_t Rotate = 0;
			for (TimeBit = 0; TimeBit < SymbolsPerBlock; TimeBit++) {
				if (FHT_Buffer[TimeBit] < 0) {
					size_t Bit = FreqBit+Rotate;
					if (Bit >= BitsPerSymbol) Bit -= BitsPerSymbol;
					uint8_t Mask = 1;
					Mask <<= Bit;
					OutputBlock[TimeBit] |= Mask;
				}
				Rotate += 1;
				if (Rotate >= BitsPerSymbol)
					Rotate -= BitsPerSymbol;
			}
		}
	}

	void PrintOutputBlock(void)
		{ size_t TimeBit;
		for (TimeBit=0; TimeBit<SymbolsPerBlock; TimeBit++)
		{ printf("%2d: ",(int)TimeBit);
			PrintBinary(OutputBlock[TimeBit],BitsPerSymbol);
			printf("\n"); }
		}
	
} ;

template <class InpType, class CalcType>
 class MFSK_SoftDecoder
{
public:

	size_t BitsPerSymbol;		// number of bits per symbol
	size_t BitsPerCharacter;	// number of bits per character
	size_t Symbols;				// number of symbols
	size_t SymbolsPerBlock;	// number of symbols per FEC block
	float Signal, NoiseEnergy;
	uint8_t *OutputBlock;

	bool	bContestia;
//	bool	bRTTYM;

private:
   static const uint64_t ScramblingCodeOlivia = 0xE257E6D0291574ECLL;
   static const uint64_t ScramblingCodeContestia = 0xEDB88320LL;
//   static const uint64_t ScramblingCodeRTTYM = 0xEDB88320LL;
   uint64_t ScramblingCode;

	size_t InputBufferLen;
	InpType *InputBuffer;
	size_t InputPtr;

	CalcType *FHT_Buffer;

public:

	MFSK_SoftDecoder() {
			bContestia = false;
			Init();
			Default();
	}

	~MFSK_SoftDecoder() {
			Free();
	}

	void Default(void) {
		bContestia = false;
		BitsPerSymbol = 5;
		BitsPerCharacter = 7;
		ScramblingCode = ScramblingCodeOlivia;
	}

	void Init(void) {
			InputBuffer = 0;
			FHT_Buffer = 0;
			OutputBlock = 0;
	}

	void Free(void) {
			delete [] InputBuffer;
			InputBuffer = 0;
			delete [] FHT_Buffer;
			FHT_Buffer = 0;
			delete [] OutputBlock;
			OutputBlock = 0;
	}

	void Reset(void) {
			size_t Idx;
			for (Idx = 0; Idx < InputBufferLen; Idx++)
				InputBuffer[Idx] = 0;
			InputPtr = 0;
	}

	int Preset(void) {
//			if (bRTTYM) {
//				BitsPerCharacter = 5;
//				ScramblingCode = ScramblingCodeRTTYM;
//			} else 
		if (bContestia) {
			BitsPerCharacter = 6;
			ScramblingCode = ScramblingCodeContestia;
		} else {
			BitsPerCharacter = 7;
			ScramblingCode = ScramblingCodeOlivia;
		}

		Symbols = 1 << BitsPerSymbol;
		SymbolsPerBlock = Exp2(BitsPerCharacter - 1);
		InputBufferLen = SymbolsPerBlock * BitsPerSymbol;
		if (ReallocArray(&InputBuffer, InputBufferLen) < 0) goto Error;
		if (ReallocArray(&FHT_Buffer, SymbolsPerBlock) < 0) goto Error;
		if (ReallocArray(&OutputBlock, BitsPerSymbol) < 0) goto Error;
		Reset();
		return 0;
	Error:
		Free();
		return -1;
	}

	int Preset(MFSK_SoftDecoder<InpType,CalcType> &RefDecoder) {
			BitsPerSymbol = RefDecoder.BitsPerSymbol;
//			BitsPerCharacter = RefDecoder.BitsPerCharacter;
			return Preset();
	}

	void Input(InpType *Symbol) {
			size_t FreqBit;
			for (FreqBit = 0; FreqBit < BitsPerSymbol; FreqBit++) {
				InputBuffer[InputPtr] = Symbol[FreqBit];
				InputPtr += 1;
			}
			if (InputPtr >= InputBufferLen) InputPtr -= InputBufferLen;
	}

	void DecodeCharacter(size_t FreqBit) {
		size_t TimeBit;
		size_t Ptr = InputPtr;
		size_t Rotate = FreqBit;
		size_t CodeWrap = (SymbolsPerBlock - 1);
		// Olivia (13 bit shift) or Contestia/RTTYM (5 bit shift)
//		size_t nShift	= (bContestia || bRTTYM) ? 5 : 13;
		size_t nShift	= (bContestia) ? 5 : 13;

		size_t CodeBit = FreqBit * nShift;

		CodeBit &= CodeWrap;
		for (TimeBit = 0; TimeBit < SymbolsPerBlock; TimeBit++) {
			InpType Bit = InputBuffer[Ptr + Rotate];
			uint64_t CodeMask = 1;
			CodeMask <<= CodeBit;
			if (ScramblingCode & CodeMask) Bit = (-Bit);
			FHT_Buffer[TimeBit] = Bit;
			CodeBit += 1;
			CodeBit &= CodeWrap;
			Rotate += 1;
			if (Rotate >= BitsPerSymbol) Rotate -= BitsPerSymbol;
			Ptr += BitsPerSymbol;
			if (Ptr >= InputBufferLen) Ptr -= InputBufferLen;
		}
		FHT(FHT_Buffer, SymbolsPerBlock);
		CalcType Peak = 0;
		size_t PeakPos = 0;
		CalcType SqrSum = 0;
		for (TimeBit = 0; TimeBit < SymbolsPerBlock; TimeBit++) {
			CalcType Signal = FHT_Buffer[TimeBit];
			SqrSum += Signal * Signal;
			if (fabs(Signal) > fabs(Peak)) {
				Peak = Signal;
				PeakPos = TimeBit;
			}
		}
		uint8_t Char = PeakPos;
		if (Peak < 0) Char += SymbolsPerBlock;
		SqrSum -= Peak * Peak;

		if (bContestia && Char > 0) {
			if (Char == 59)
				Char = ' ';
			else if (Char == 60)
				Char = '\r';
			else if (Char == 61)
				Char = 8; // backspace
			else
				Char += 32;
		}
		OutputBlock[FreqBit] = Char;

		NoiseEnergy += (float)SqrSum / (SymbolsPerBlock - 1);
		Signal += fabs(Peak);
	}

	void Process(void) {
			size_t FreqBit;
			Signal = 0;
			NoiseEnergy = 0;
			for (FreqBit = 0; FreqBit < BitsPerSymbol; FreqBit++)
				DecodeCharacter(FreqBit);
			Signal /= BitsPerSymbol;
			NoiseEnergy /= BitsPerSymbol;
	}

	size_t Output(uint8_t *Buffer) {
			size_t FreqBit;
			for (FreqBit = 0; FreqBit < BitsPerSymbol; FreqBit++)
				Buffer[FreqBit] = OutputBlock[FreqBit];
			return BitsPerSymbol;
	}

	size_t Output(uint64_t &PackedBuffer) {
			size_t FreqBit;
			PackedBuffer = 0;
			for (FreqBit = BitsPerSymbol; FreqBit > 0; ) {
				PackedBuffer <<= 8;
				FreqBit--;
				PackedBuffer |= OutputBlock[FreqBit];
			}
			return BitsPerSymbol;
	}

	void PrintOutputBlock(FILE *File = stdout) {
			size_t FreqBit;
			fprintf(File, "'");
			for (FreqBit = 0; FreqBit < BitsPerSymbol; FreqBit++) {
				uint8_t Char = OutputBlock[FreqBit];
				fprintf(File, "%c", (Char >= ' ') && (Char < 127) ? Char:' ');
			}
			fprintf(File, "'");
			if (NoiseEnergy > 0)
				fprintf(File, ", S/N = %5.1f", Signal / sqrt(NoiseEnergy));
			fprintf(File, "\n");
	}

	size_t Output(uint64_t *PackedBuffer) {
			return Output(*PackedBuffer);
	}

};

// =====================================================================

// rate converter
template <class Type=float>
class RateConverter
{
public:
// parameters to be set by the user
	size_t	TapLen;		// filter tap length (in term of input samples)
	size_t	OverSampling; // internal oversampling factor
	Type	UpperFreq;	// upper frequency of the (lowpass) filter (in terms of input sampling rate)
	Type	OutputRate;	// the output rate (in terms of the input rate)

private:
	size_t FilterLen;	// the total length of the filter (in term of oversampled rate)
	Type *FilterShape;	// the shape of the filter
	Type *InputTap;		// filter tap
	size_t InputTapPtr;
	size_t InputWrap;

	Type OutputTime;
	Type OutputPeriod;
	Type OutputBefore;
	Type OutputAfter;
	size_t OutputPtr;
	
public:

	RateConverter() {
			Init();
			Default();
	}
		
	~RateConverter() {
			Free();
	}

	void Init(void) {
			FilterShape = 0;
			InputTap = 0;
	}

	void Free(void) {
			delete [] FilterShape;
			FilterShape = 0;
			delete [] InputTap;
			InputTap = 0;
	}

	void Default(void) {
			TapLen = 16;
			OverSampling = 16;
			UpperFreq = 3.0 / 8;
			OutputRate = 1.0;
	}

	int Preset(void) {
			size_t Idx;

			TapLen = Exp2(Log2(TapLen));
			FilterLen = TapLen * OverSampling;

			if ((ReallocArray(&FilterShape, FilterLen)) < 0) goto Error;
			if ((ReallocArray(&InputTap, TapLen)) < 0) goto Error;

			for (Idx = 0; Idx < FilterLen; Idx++) {
				Type Phase = (M_PI * (2 * (int)Idx - (int)FilterLen)) / FilterLen;
			// Hanning 
			//	Type Window = 0.50 + 0.50 * cos(Phase);
			// Blackman
			//	Type Window = 0.42 + 0.50 * cos(Phase) + 0.08 * cos(2 * Phase);
			// Blackman-Harris
				Type Window = 0.35875 + 0.48829 * cos(Phase) +
									0.14128 * cos(2 * Phase) + 0.01168 * cos(3 * Phase);
				Type Filter = 1.0;
				if (Phase != 0) {
					Phase *= (UpperFreq * TapLen);
					Filter = sin(Phase) / Phase;
				}
		// printf("%3d: %+9.6f %+9.6f %+9.6f\n", Idx, Window, Filter, Window*Filter);
				FilterShape[Idx] = Window * Filter;
			}
			Reset();
			return 0;
	Error:
			Free();
			return -1;
	}

	void Reset(void) {
			size_t Idx;

			InputWrap = TapLen - 1;
			for (Idx = 0; Idx < TapLen; Idx++)
				InputTap[Idx] = 0;
			InputTapPtr = 0;

			OutputTime = 0;
			OutputPeriod = OverSampling / OutputRate;
			OutputBefore = 0;
			OutputAfter = 0;
			OutputPtr = 0;
	}

private:
	
	Type Convolute(size_t Shift=0) {
			Type Sum = 0;
			Shift = (OverSampling - 1) - Shift;
			size_t Idx = InputTapPtr;
			for ( ; Shift < FilterLen; Shift += OverSampling) {
				Sum += InputTap[Idx] * FilterShape[Shift];
				Idx += 1;
				Idx &= InputWrap;
			}
			return Sum;
	}

	void NewInput(Type Input)
		{ // printf("I:\n");
		InputTap[InputTapPtr]=Input;
		InputTapPtr+=1; InputTapPtr&=InputWrap; }

public:

	template <class InpType, class OutType>
	int Process(InpType *Input, size_t InputLen, OutType *Output) {
			size_t OutputLen = 0;
		// printf("E: %d %3.1f %d %d\n",OutputPtr, OutputTime, InputLen, OutputLen);
			for ( ; ; ) {
				// printf("L: %d %3.1f %d %d\n",OutputPtr, OutputTime, InputLen, OutputLen);
				if (OutputPtr) {
					size_t Idx = (size_t)floor(OutputTime) + 1;
					if (Idx >= OverSampling) {
							if (InputLen == 0) break;
							NewInput(*Input);
							Input++;
							InputLen -= 1;
							Idx -= OverSampling;
							OutputTime -= (Type)OverSampling;
					}
					OutputAfter = Convolute(Idx);
					Type Weight = Idx - OutputTime;
					(*Output) = Weight * OutputBefore + (1.0 - Weight) * OutputAfter;
					Output++;
					OutputLen += 1;
					// printf("O: %d %3.1f %d %d %d\n",OutputPtr, OutputTime, InputLen, OutputLen, Idx);
					OutputPtr = 0;
				} else {
					size_t Idx = (size_t)floor(OutputTime + OutputPeriod);
					if (Idx >= OverSampling) {
							if (InputLen == 0) break;
							NewInput(*Input);
							Input++;
							InputLen -= 1;
							Idx -= OverSampling;
							OutputTime -= (Type)OverSampling;
					}
					OutputBefore = Convolute(Idx);
					OutputTime += OutputPeriod;
					OutputPtr = 1;
				}
			}
			// printf("R: %d %3.1f %d %d\n",OutputPtr, OutputTime, InputLen, OutputLen);
			return OutputLen;
	}

	template <class InpType, class OutType>
	int Process(InpType Input, size_t InputLen, Seq<OutType> &Output) {
			size_t OutPtr = Output.Len;
			size_t MaxOutLen = (size_t)ceil(InputLen * OutputRate + 2);
			if (Output.EnsureSpace(OutPtr + MaxOutLen) < 0) return -1;
			int OutLen = Process(Input, InputLen, Output.Elem+OutPtr);
			Output.Len += OutLen;
			return OutLen;
	}

	template <class InpType, class OutType>
	int Process(InpType Input, OutType *Output) {
			return Process(&Input, 1, Output);
	}

};

// =====================================================================

template <class Type=float>
class MFSK_Transmitter
{
public:
// primary parameters: set by the user
	bool	bContestia;
	size_t Tones;			// number of tones: 4, 8, 16, 32, 64, 128, 256
	size_t Bandwidth;		// bandwidth: 125, 250, 500, 1000, 2000
	Type	SampleRate;		// audio sampling rate (internal processing)
	Type OutputSampleRate; // true sampling rate of the soundcard
	float FirstCarrierMultiplier;
	int Reverse;
// secondary parameters: calculated by Preset()
	size_t BitsPerSymbol;	// number of bits per symbol
	size_t SymbolsPerBlock; // number of symbols per one FEC code block
	size_t MaxOutputLen;	// maximum length of the audio batch returned by Output()

private:
	static const int State_Running = 0x0001;
	static const int State_StopReq = 0x0010;
	int State;

	FIFO<uint8_t> Input;		// buffer(queue) for the characters to be encoded
	uint8_t InputBlock[8];	// FEC code block buffer
	FIFO<uint8_t> Monitor;	// buffer for monitoring the characters being sent

	MFSK_Encoder Encoder;	// FEC encoder
	size_t SymbolPtr;

	MFSK_Modulator<Type> Modulator; // MFSK modulator

	Type *ModulatorOutput;

	RateConverter<Type> Converter; // output rate converter

	Type *ConverterOutput;

public:

	MFSK_Transmitter() {
			bContestia = false;
			Init();
	}

	~MFSK_Transmitter() {
			Free();
	}

	void Init(void) {
			ModulatorOutput = 0;
			ConverterOutput = 0;
	}

	void Free(void) {
			Input.Free();
			Monitor.Free();
			Encoder.Free();
			Modulator.Free();
			delete [] ModulatorOutput; 
			ModulatorOutput = 0;
			Converter.Free();
			delete [] ConverterOutput; 
			ConverterOutput = 0;
	}

// set default primary parameters
	void Default(void) {
			Tones = 32;
			Bandwidth = 1000;
			Reverse = 0;
			SampleRate = 8000;
			OutputSampleRate = 8000;
	}

// preset internal arrays according to primary paramaters
	int Preset(void) {
	// impose limits on the primary parameters
			if (Tones < 2 ) Tones = 2;
			else if (Tones > 256) Tones = 256;
			if (Bandwidth < 125) Bandwidth = 125;
			else if (Bandwidth > 2000) Bandwidth = 2000;

	// calculate the secondary parameters
			BitsPerSymbol = Log2(Tones);
			Tones = Exp2(BitsPerSymbol);

	// preset the input character buffer
			Input.Len = 1024;
			if (Input.Preset() <0 ) goto Error;
			Monitor.Len = 256;
			if (Monitor.Preset() < 0) goto Error;

	// preset the encoder
			Encoder.bContestia = bContestia;
			Encoder.BitsPerSymbol = BitsPerSymbol;
			if (Encoder.Preset() < 0) goto Error;
			SymbolsPerBlock = Encoder.SymbolsPerBlock;

	// preset the modulator
			Modulator.Default();
			Modulator.BitsPerSymbol = BitsPerSymbol;
			Modulator.SymbolLen = 
				(size_t)1 << (BitsPerSymbol + 7 - Log2(Bandwidth / 125));
			Bandwidth = Exp2(Log2(Bandwidth / 125)) * 125;
			Modulator.FirstCarrier =
				(size_t) ((Modulator.SymbolLen / 16) * FirstCarrierMultiplier) + 1;
			Modulator.Reverse = Reverse;
			if (Modulator.Preset() < 0) goto Error;

			if (ReallocArray(&ModulatorOutput, Modulator.SymbolSepar) < 0)
				goto Error;

	// preset the rate converter
			Converter.OutputRate = OutputSampleRate / SampleRate;
			if (Converter.Preset() < 0) goto Error;

			MaxOutputLen = 
				(size_t)ceil(Modulator.SymbolSepar * OutputSampleRate / SampleRate + 2);
			if (ReallocArray(&ConverterOutput, MaxOutputLen) < 0) goto Error;

	// reset the state logic
			SymbolPtr = 0;
			State = 0;

			return 0;

	Error:
			Free(); 
			return -1;
	}

	void Reset(void) {
			Input.Reset();
			Monitor.Reset();
			SymbolPtr = 0;
			State = 0;
			Converter.Reset();
	}

	Type BaudRate(void) {
			return SampleRate / Modulator.SymbolSepar;
	}

	
	Type BlockPeriod(void) {
		return (SymbolsPerBlock * Modulator.SymbolSepar) / SampleRate;
	}

	Type CharactersPerSecond(void) {
			return BitsPerSymbol * SampleRate /
					(SymbolsPerBlock * Modulator.SymbolSepar);
	}

// start the transmission
	void Start(void) {
			State |= State_Running;
	}

// request to stop (and complete) the transmission
// but the transmitter will only stop after transmitting all the data
	void Stop(void) {
			State |= State_StopReq;
	}

// check if the transmission is still running (not complete)
	int Running(void) {
			return State & State_Running;
	}

// put the character into the transmitter input queue
	int PutChar(uint8_t Char) {
			return Input.Write(Char);
	}

// get one character from the monitor buffer
	int GetChar(uint8_t &Char) {
			return Monitor.Read(Char);
	}

	size_t GetReadReady(void) {
			return Input.ReadReady();
	}

	int DoPostambleYet(void) {
			if (State == 0)
				return 1; 
			else
				return 0;
	}

	double GetSymbolPhase(void) {
			return Modulator.SymbolPhase;
	}

// get out the transmitter output (audio)
	int Output(double *Buffer) {
			if (SymbolPtr == 0) {
				if ((State&State_StopReq) && Input.Empty()) {
					State=0;
				} else if (State&State_Running) {
					size_t Idx;
					for (Idx = 0; Idx < BitsPerSymbol; Idx++) {
							uint8_t Char;
							if (Input.Read(Char) <= 0) break;
							InputBlock[Idx] = Char;
							Monitor.Write(Char);
					}
					for ( ; Idx < BitsPerSymbol; Idx++)
							InputBlock[Idx] = 0;
					Encoder.EncodeBlock(InputBlock);
				}
			}
			if (State&State_Running) {
				Modulator.Send(Encoder.OutputBlock[SymbolPtr]);
				SymbolPtr += 1;
				if (SymbolPtr >= SymbolsPerBlock) SymbolPtr = 0;
			}
			int ModLen = Modulator.Output(ModulatorOutput);
			int ConvLen = Converter.Process(ModulatorOutput, ModLen, ConverterOutput);
			if (ConvLen < 0) return ConvLen;
		
			double maxval = 0, tempnum;
			for (int Idx = 0; Idx < ConvLen; Idx++)
				if ((tempnum = fabs(ConverterOutput[Idx])) > maxval)
					maxval = tempnum;
			for (int Idx = 0; Idx < ConvLen; Idx++)
				Buffer[Idx] = (double) ConverterOutput[Idx] / maxval;

			return ConvLen; 
	}

};

// =====================================================================

/*

How to use the MFSK_Receiver class:

1. create an object like:

	#include "mfsk.h"

	MFSK_Receiver<float> Receiver;

2. Set the parameters, for example:

	Receiver.Tones				= 32;		// number of tones (symbols)
	Receiver.Bandwidth		= 1000;	// bandwidth [Hz]
	Receiver.SyncMargin		= 8;		// synchronizer tune margin [tone freq. spacing]
	Receiver.SyncIntegLen	= 4;		// synchronizer integration period [FEC blocks]
	Receiver.SyncThreshold	= 3.2;	// S/N threshold for printing
	Receiver.SampleRate		= 8000.0; // internal processor sampling rate [Hz]
	Receiver.InputSampleRate = 8000.0; // soundcard sampling rate [Hz]

	You don't need to set all the parameters, as upon creation
	of the Receiver object they are already given certain default
	values.

	If you changed parameters at one time and want later to go back
	to the default values you can call: Receiver.Default();

3. Preset the Receiver internal arrays for the parameters you just set:

	if (Receiver.Preset()<0)
		printf("Not enough RAM or another problem\n");

	Each time you change the parameters you need to call Preset()
	in order to resize the internal arrays. Preset() will as well
	destroy all data being in the process of decoding, if you need
	this data then call first Receiver.Flush()

4. Read back the parameters you set in point 1., they could have been adjusted
	by Preset() to their closest allowed values.

5. Feed the audio into the Receiver:

	Receiver.Process(AudioBuffer, BufferLength);

	AudioBuffer can be an array of int16_t (16-bit signed integers)
	that you fill with the data from the soundcard. I suggest you feed
	the receiver with batches of 512 or 1024 samples, but in can be any number
	of samples at a time.

6. Call GetChar(Char) to get decoded characters. Note, that
	characters come in batches, and so, you need to call GetChar()
	possibly several times. GetChar() returns 0 when the decoder FIFO
	is empty or 1 when not empty. In the latter case the argument
	contains the character read form the FIFO. The loop could be like:
	
	for ( ; ; )
	{ uint8_t Char;
		if (Receiver.GetChar(Char)==0) break;
		printf("%c",Char);
	}

	Keep in mind that you may see (random) control code characters here,
	and thus you must be able to deal with them. I suggest to process
	only carriage return (code=13) and Backspace (code=8). NUL (code=0)
	is the idle character: it is being sent when there is no text to be sent.

7. At any time you can read the signal-to-noise ratio of the
	incoming signal by calling Receiver.SignalToNoiseRatio() or frequency offset
	by calling Receiver.FrequencyOffset()

8. When the user decides to turn off the receiver and switch over
	to transmitt you may still call Receiver.Flush()in order to flush
	the data still being buffered in the decoder pipeplines.

*/

template <class Type=float>
class MFSK_Receiver
{
public:
// primary parameters: set by the user

	bool	bContestia;
	size_t Tones;					// number of tones: 4, 8, 16, 32, 64, 128, 256
	size_t Bandwidth;				// bandwidth: 125, 250, 500, 1000, 2000
	size_t SyncMargin;				// synchronizer search margin, frequency-wide
	size_t SyncIntegLen;			// synchronizer integration period
	Type	SyncThreshold;			// synchronizer S/N threshold
	Type	SampleRate;				// audio sampling rate (internal processing)
	Type	InputSampleRate;		// true sampling rate of the soundcard
	float	FirstCarrierMultiplier;
	int	Reverse;

// secondary parameters: calculated by Preset()
	size_t BitsPerSymbol;		// number of bits per symbol
	size_t SymbolsPerBlock;		// number of symbols per one FEC code block

private:
	RateConverter<Type> Converter;
	Seq<Type> InputBuffer;
	MFSK_InputProcessor<Type> InputProcessor;	// equalizes the input spectrum
												// and removes coherent interferences
	MFSK_Demodulator<Type> Demodulator;			// FFT demodulator

	const static size_t SlicesPerSymbol = 
			MFSK_Demodulator<Type>::SpectraPerSymbol;

// number of possible frequency offsets
	size_t FreqOffsets;
// number of possible time-phases within the FEC block
	size_t BlockPhases;
// reference decoder
	MFSK_SoftDecoder<Type,Type> RefDecoder;
// array of decoders
	MFSK_SoftDecoder<Type,Type> *Decoder;
// current running time-phase
	size_t BlockPhase;
// FEC noise integrators
	CircularBuffer< LowPass3_Filter<Type> >	SyncNoiseEnergy;
// FEC signal integrators
	CircularBuffer< LowPass3_Filter<Type> >	SyncSignal;
// weight for the integrators
	Type SyncFilterWeight;
// best signal
	Type SyncBestSignal;
// time-phase of the best signal
	size_t SyncBestBlockPhase;
// frequency offset of the best signal
	size_t SyncBestFreqOffset;
// S/N corresponding to the SyncBestSignal
	Type SyncSNR;
// pipeline for decoded FEC blocks
	CircularBuffer<uint64_t> *DecodePipe;
// buffer for decoded characters
	FIFO<uint8_t> Output;

public:
	MFSK_Receiver() {
			bContestia = false;
			Init();
			Default();
	}
	~MFSK_Receiver() {
			Free();
	}
	void Init(void) {
			Decoder = 0;
			DecodePipe = 0;
	}
	void Free(void) {
			if (Decoder) {
				size_t Idx;
				for (Idx = 0; Idx < (SlicesPerSymbol * FreqOffsets); Idx++)
					Decoder[Idx].Free();
				delete [] Decoder; Decoder=0;
			}
			if (DecodePipe) {
				size_t Idx;
				for (Idx = 0; Idx < BlockPhases; Idx++)
					DecodePipe[Idx].Free();
				delete [] DecodePipe; 
				DecodePipe = 0;
			}
			Converter.Free();
			InputBuffer.Free();
			InputProcessor.Free();
			Demodulator.Free();
			RefDecoder.Free();
			SyncSignal.Free();
			SyncNoiseEnergy.Free();
			Output.Free();
	}

// set defaults values for all parameters
	void Default(void) {
			Tones = 32;
			Reverse = 0;
			Bandwidth = 1000;
			SyncMargin = 8;
			SyncIntegLen = 4;
			SyncThreshold = 3.0;
			SampleRate = 8000;
			InputSampleRate = 8000.0; 
	}

// resize internal arrays according the parameters
	int Preset(void) {
			size_t Idx;
			if (Tones < 2) Tones = 2;
			else if (Tones > 256) Tones = 256;
			
			if (Bandwidth < 125) Bandwidth = 125;
			else if (Bandwidth > 2000) Bandwidth = 2000;

			if (SyncMargin < 2) SyncMargin = 2;

			if (SyncIntegLen < 2) SyncIntegLen = 2;

			if (SyncThreshold < 3.0) SyncThreshold = 3.0;

			BitsPerSymbol = Log2(Tones);
			Tones = Exp2(BitsPerSymbol);

			Converter.OutputRate = SampleRate/InputSampleRate;
			if (Converter.Preset() < 0) goto Error;

			Demodulator.BitsPerSymbol = BitsPerSymbol;
			Demodulator.SymbolLen = Exp2(BitsPerSymbol + 7 - Log2(Bandwidth/125));
			Bandwidth = Exp2(Log2(Bandwidth/125)) * 125;
			Demodulator.FirstCarrier = 
				(size_t) ((Demodulator.SymbolLen/16) * FirstCarrierMultiplier) + 1;
			Demodulator.Reverse = Reverse;
			Demodulator.DecodeMargin = SyncMargin;
			if (Demodulator.Preset() < 0) goto Error;
			SyncMargin = Demodulator.DecodeMargin;

			InputProcessor.WindowLen = 16 * Demodulator.SymbolLen;
			if (InputProcessor.Preset() < 0) goto Error;

		RefDecoder.Default();
		RefDecoder.BitsPerSymbol=BitsPerSymbol;
		RefDecoder.bContestia = bContestia;
		if (RefDecoder.Preset()<0) goto Error;
		SymbolsPerBlock=RefDecoder.SymbolsPerBlock;

			if (Decoder) {
				for (Idx = 0; Idx < (SlicesPerSymbol * FreqOffsets); Idx++)
					Decoder[Idx].Free();
			}

			if (DecodePipe) {
				for (Idx = 0; Idx < BlockPhases; Idx++)
				DecodePipe[Idx].Free();
			}

			FreqOffsets = 2 * SyncMargin + 1;
			BlockPhases = SlicesPerSymbol * SymbolsPerBlock;

			if (ReallocArray(&Decoder, SlicesPerSymbol * FreqOffsets) < 0) 
				goto Error;

			for (Idx = 0; Idx < (SlicesPerSymbol * FreqOffsets); Idx++) {
				Decoder[Idx].bContestia = bContestia;
				Decoder[Idx].Init();
			}

			for (Idx = 0; Idx < (SlicesPerSymbol * FreqOffsets); Idx++)
				if (Decoder[Idx].Preset(RefDecoder) < 0) goto Error;

			if (ReallocArray(&DecodePipe,BlockPhases) < 0) goto Error;
			for (Idx = 0; Idx < BlockPhases; Idx++)
				DecodePipe[Idx].Init();
			for (Idx = 0; Idx < BlockPhases; Idx++) {
				DecodePipe[Idx].Width = FreqOffsets;
				DecodePipe[Idx].Len = SyncIntegLen;
				if (DecodePipe[Idx].Preset() < 0) goto Error;
				DecodePipe[Idx].Clear();
			}

			SyncSignal.Width = FreqOffsets;
			SyncSignal.Len = BlockPhases;
			if (SyncSignal.Preset() < 0) goto Error;
			SyncSignal.Clear();

			SyncNoiseEnergy.Width = FreqOffsets;
			SyncNoiseEnergy.Len = BlockPhases;
			if (SyncNoiseEnergy.Preset() < 0) goto Error;
			SyncNoiseEnergy.Clear();

			SyncFilterWeight = 1.0 / SyncIntegLen;

			BlockPhase=0;

			SyncBestSignal = 0;
			SyncBestBlockPhase = 0;
			SyncBestFreqOffset = 0;
			SyncSNR = 0;

			if (InputBuffer.EnsureSpace(InputProcessor.WindowLen + 2048) < 0)
				goto Error;

			Output.Len = 1024;
			if (Output.Preset() < 0) goto Error;

			return 0;

	Error:
			Free();
			return -1;
	}

	void Reset(void) {
			size_t Idx;
			Converter.Reset();
			InputBuffer.Clear();
			InputProcessor.Reset();
			for (Idx = 0; Idx < (SlicesPerSymbol * FreqOffsets); Idx++)
				Decoder[Idx].Reset();

			for (Idx = 0; Idx < BlockPhases; Idx++)
				DecodePipe[Idx].Clear();

			SyncSignal.Clear();
			SyncNoiseEnergy.Clear();

			BlockPhase = 0;

			SyncBestSignal = 0;
			SyncBestBlockPhase = 0;
			SyncBestFreqOffset = 0;
			SyncSNR = 0;
 
			Output.Reset();
	}

	char *PrintParameters(void) {
		static char szParams[1000];
		Type FreqBin = SampleRate/Demodulator.SymbolLen;
		snprintf(szParams, sizeof(szParams), "\
%d tones, %4.2f Hz/tone, %4.1f Hz bandwidth\n\
%d bits/symbol(tone), %4.2f baud\n\
FEC block: %d char. => %d x %d bits, %3.1f sec\n\
Audio band: %3.1f - %3.1f Hz, %3.1f Hz total\n\
Tuning tolerance = +/- %3.1f Hz\n\
Sync. S/N threshold = %3.1f\n",
		(int)Demodulator.Carriers,
		FreqBin*Demodulator.CarrierSepar,
		FreqBin*Demodulator.CarrierSepar*Demodulator.Carriers,
		(int)Demodulator.BitsPerSymbol,
		SampleRate/Demodulator.SymbolSepar,
		(int)RefDecoder.BitsPerSymbol,
		(int)RefDecoder.BitsPerSymbol, 
		(int)RefDecoder.SymbolsPerBlock,
		(SymbolsPerBlock*Demodulator.SymbolSepar)/SampleRate,
		FreqBin * Demodulator.FirstCarrier,
		FreqBin * (Demodulator.FirstCarrier + Demodulator.CarrierSepar * Demodulator.Carriers),
		FreqBin * Demodulator.CarrierSepar * Demodulator.Carriers,
		FreqBin * SyncMargin, 
		SyncThreshold );
		return szParams;
	}

	Type BaudRate(void) {
			return SampleRate / Demodulator.SymbolSepar;
	}

	Type TuneMargin(void) { 
			return SyncMargin * (SampleRate / Demodulator.SymbolLen);
	}

	Type BlockPeriod(void) {
			return (SymbolsPerBlock * Demodulator.SymbolSepar) / SampleRate;
	}

	Type CharactersPerSecond(void) {
			return BitsPerSymbol * SampleRate / 
					(SymbolsPerBlock*Demodulator.SymbolSepar);
	}

	Type SignalToNoiseRatio(void) {
			return SyncSNR;
	}

	Type FrequencyOffset(void) {
			return ( (int)SyncBestFreqOffset - 
						(int)FreqOffsets / 2) * (SampleRate / Demodulator.SymbolLen);
	}

	Type TimeOffset(void) {
			return ( (Type)SyncBestBlockPhase / SlicesPerSymbol) * 
						(Demodulator.SymbolSepar / SampleRate);
	}

// process an audio batch: first the input processor, then the demodulator
	template <class InpType>
	int Process(InpType *Input, size_t InputLen) {
			if (Converter.Process(Input, InputLen, InputBuffer) < 0)
				return -1;
			ProcessInputBuffer();
			return 0;
	}

	void Flush(void) {
			ProcessInputBuffer();
			size_t Idx;

			for (Idx = InputBuffer.Len; Idx < InputProcessor.WindowLen; Idx++)
				InputBuffer[Idx] = 0;
			InputBuffer.Len = InputProcessor.WindowLen;
			ProcessInputBuffer();

			for (Idx = 0; Idx < InputProcessor.WindowLen; Idx++)
				InputBuffer[Idx] = 0;
			size_t FlushLen = Demodulator.SymbolSepar * SymbolsPerBlock * 
									SyncIntegLen * 2;
			for (Idx=0; Idx<FlushLen; Idx+=InputProcessor.WindowLen) {
				InputBuffer.Len = InputProcessor.WindowLen;
				ProcessInputBuffer();
			}
	}

// get one character from the output buffer
	int GetChar(uint8_t &Char) {
			return Output.Read(Char);
	}

private:
// process the input buffer: first the input processor, then the demodulator
	void ProcessInputBuffer(void) {
			while(InputBuffer.Len >= InputProcessor.WindowLen) {
				InputProcessor.Process(InputBuffer.Elem);
				InputBuffer.Delete(0, InputProcessor.WindowLen);
				size_t Idx;
				for (Idx = 0; 
					Idx < InputProcessor.WindowLen; 
					Idx += Demodulator.SymbolSepar )
					ProcessSymbol(InputProcessor.Output+Idx);
			}
	}

// process (through the demodulator) an audio batch corresponding to one symbol
// demodulator always uses audio batches corresponding to one symbol period
	template <class InpType>
	void ProcessSymbol(InpType *Input) {
			Demodulator.Process(Input);
			MFSK_SoftDecoder<Type,Type> *DecoderPtr = Decoder;

			size_t Offset,Slice;
			for (Slice = 0; Slice < SlicesPerSymbol; Slice++) {
				LowPass3_Filter<Type> 
					*NoiseEnergyPtr = SyncNoiseEnergy.AbsPtr(BlockPhase);
				LowPass3_Filter<Type> 
					*SignalPtr = SyncSignal.AbsPtr(BlockPhase);

				uint64_t *DecodeBlockPtr = DecodePipe[BlockPhase].CurrPtr();

				size_t	BestSliceOffset = 0;
				Type	BestSliceSignal = 0;
				Type	NoiseEnergy = 0;
				Type	Signal;
				Type	Symbol[8];

				for (Offset = 0; Offset < FreqOffsets; Offset++) {
					Demodulator.SoftDecode(Symbol, Slice, (int)Offset - (FreqOffsets / 2));

					DecoderPtr->Input(Symbol);
					DecoderPtr->Process();
					DecoderPtr->Output(DecodeBlockPtr);

					NoiseEnergy = DecoderPtr->NoiseEnergy;
					NoiseEnergyPtr->Process(NoiseEnergy, SyncFilterWeight);

					Signal = DecoderPtr->Signal;
					SignalPtr->Process(Signal, SyncFilterWeight);
					Signal = SignalPtr->Output;

					if (Signal > BestSliceSignal) {
							BestSliceSignal = Signal;
							BestSliceOffset = Offset;
					}

					DecoderPtr++;
					DecodeBlockPtr++;

					NoiseEnergyPtr++;
					SignalPtr++;
				}
				DecodePipe[BlockPhase] += 1;

				if (BlockPhase == SyncBestBlockPhase) {
					SyncBestSignal = BestSliceSignal;
					SyncBestFreqOffset = BestSliceOffset;
				} else {
					if (BestSliceSignal > SyncBestSignal) {
							SyncBestSignal = BestSliceSignal;
							SyncBestBlockPhase = BlockPhase;
							SyncBestFreqOffset = BestSliceOffset;
					}
				}

				int Dist = (int)BlockPhase - (int)SyncBestBlockPhase;
				if (Dist < 0) Dist += BlockPhases;

				if (Dist == (int)(BlockPhases / 2)) {
					Type BestNoise = sqrt( NoiseEnergyPtr->Output);
					if (BestNoise == 0) SyncSNR = 0;
					else SyncSNR = SyncBestSignal / BestNoise;
//					printf(
//					"%d, %6.3f/%6.3f = %5.2f\n", 
//							SyncBestFreqOffset,
//							SyncBestSignal, 
//							BestNoise, 
//							SyncSNR );
					if (SyncSNR >= SyncThreshold) {
							uint64_t *BestBlockPtr = 
											DecodePipe[SyncBestBlockPhase].CurrPtr();
							uint64_t Block = BestBlockPtr[SyncBestFreqOffset];

							for ( size_t Byte = 0; Byte < BitsPerSymbol; Byte++) {
								uint8_t Char = Block&0xFF;
								Output.Write(Char);
								Block >>= 8; 
							}
					}
					if (SyncSNR > 100) SyncSNR = 0.0;
				}

				BlockPhase++;
			}
			if (BlockPhase >= BlockPhases) BlockPhase -= BlockPhases;
	}

};

#endif
