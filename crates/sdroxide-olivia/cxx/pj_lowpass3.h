// (c) 1999-2003, Pawel Jalocha
// (c) 2019, David Freese, W1HKJ

#ifndef __LOWPASS3_H__
#define __LOWPASS3_H__

// ==========================================================================

// IIR low pass filter for integration (averaging) purposes
// Overshoot is about 1% for Feedback=0.5, and about 1e-6 for Feedback=0.1
// Weight is 1 / PeakingTime

template <class Type>
class LowPass3_Filter { 
public:
	Type Out1, Out2, Output;
	template <class InpType, class WeightType>
	void Process(InpType Inp, WeightType Weight, WeightType Feedback = 0.1) { 
		Weight *= 2.0;
		Type DiffI1 = Inp;  DiffI1 -= Out1;
		Type Diff12 = Out1; Diff12 -= Out2;
		Type Diff23 = Out2; Diff23 -= Output;
		DiffI1 *= Weight;   Out1   += DiffI1;
		Diff12 *= Weight;   Out2   += Diff12;
		Diff23 *= Weight;   Output += Diff23;
		Diff23 *= Feedback; Out2   += Diff23; 
	}

	template <class LevelType>
	void operator = (LevelType Level) { 
		Out1 = Level; 
		Out2 = Level; 
		Output=Level;
	}

	template <class LevelType>
	void Set(LevelType Level = 0) { 
		Out1 = Level; 
		Out2 = Level; 
		Output=Level; 
	}
};


// ==========================================================================

#endif // of __LOWPASS3_H__

