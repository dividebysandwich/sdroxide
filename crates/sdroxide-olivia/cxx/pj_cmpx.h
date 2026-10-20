
// Complex numbers
// (c) 1999-2003, Pawel Jalocha
// (c) 2019, David Freese, W1HKJ

#ifndef __CMPX_H__
#define __CMPX_H__

#include <stdio.h>
#include <math.h>

// ----------------------------------------------------------------------------
// float/double/other-complex type

template <class Type>
class Cmpx {

public:
	 Type Re,Im;

public:

	template <class Type2>
	Cmpx<Type> const &operator=(const Cmpx<Type2> &New) {
		Re = New.Re;
		Im = New.Im;
		return (*this);
	}

	template <class Type2>
	Cmpx<Type> const &operator=(const Type2 &New) {
		Re = New;
		Im = 0;
		return (*this);
	}

	template <class Type2>
	void Set(Type2 NewRe, Type2 NewIm = 0) {
		Re = NewRe;
		Im = NewIm;
	}

	template <class Type2>
	void Set(Cmpx<Type2> New) {
		Re = New.Re;
		Im = New.Im;
	}

	void SetPhase(double Phase, double Mag = 1.0) {
		Re = Mag * cos(Phase);
		Im = Mag * sin(Phase);
	}

	void Conugate(void) {
		Im = (-Im);
	}

	void Negate(void) {
		Re = (-Re);
		Im = (-Im);
	}

	void QuarterTurnLeft(void) {
		Type OldIm = Im;
		Im = Re;
		Re = (-OldIm);
	}

	void QuarterTurnRight(void) {
		Type OldIm = Im;
		Im = (-Re);
		Re = OldIm;
	}

	double Energy(void) {
		return Re * Re + Im * Im;
	}

	double Mag2(void) {
		return Re * Re + Im * Im;
	}

	double Mag(void) {
		return sqrt(Re * Re + Im * Im);
	}

	double Phase(void) {
		return atan2(Im, Re);
	}

	int Zero(void) {
		return (Re == 0) && (Im == 0);
	}

	int NotZero(void) {
		return (Re != 0) || (Im != 0);
	}

	 // template <class Type2>
	 // void operator *= (const Type2 Mult)
	 // { Re*=Mult; Im*=Mult; }
	 // template <class Type2>
	 // void operator /= (const Type2 Mult)
	 // { Re*=Mult; Im*=Mult; }

	template <class Type2>
	double VectDotProd(Cmpx<Type2> X) {
		return Re * X.Re + Im * X.Im;
	}

	void Print(char *Form = (char *)"%+6.3f") {
		printf("[");
		printf(Form, Re);
		printf(",");
		printf(Form, Im);
		printf("]");
	}
} ;

typedef Cmpx<float> fcmpx;
typedef Cmpx<double> dcmpx;

// Some complex operators:
// at least with the BC++ they carry some overhead because
// a function is always called instead of making the code inline.

template <class type> // equal (both Re and Im) ?
inline int operator ==(Cmpx<type> &Left, Cmpx<type> &Right) {
	return (Left.Re == Right.Re) && (Left.Im == Right.Im);
}

template <class LeftType, class RightType> // equal (both Re and Im) ?
inline int operator !=(Cmpx<LeftType> &Left, Cmpx<RightType> &Right) {
	return (Left.Re != Right.Re) || (Left.Im != Right.Im);
}

template <class type> // equal ?
inline int operator ==(Cmpx<type> &Left, type &Right) {
	return (Left.Re == Right) && (Left.Im == 0);
}

template <class LeftType, class RightType> // not equal ?
inline int operator !=(Cmpx<LeftType> &Left, RightType &Right) {
	return (Left.Re != Right) || (Left.Im != 0);
}

template <class type> // bigger (magnitude) ?
inline int operator >(Cmpx<type> &Left, Cmpx<type> &Right) {
	return Left.Mag2() > Right.Mag2();
}

template <class type> // smaller (magnitude) ?
inline int operator <(Cmpx<type> &Left, Cmpx<type> &Right) {
	return Left.Mag2() < Right.Mag2();
}

template <class type> // addition to the argument on the left
inline void operator +=(Cmpx<type> &Dst, Cmpx<type> &Src) {
	Dst.Re += Src.Re;
	Dst.Im += Src.Im;
}

template <class type> // subtraction from the argument on the left
inline void operator -=(Cmpx<type> &Dst, Cmpx<type> &Src) {
	Dst.Re -= Src.Re;
	Dst.Im -= Src.Im;
}

template <class type, class num> // multiplication by a scalar
inline void operator *=(Cmpx<type> &Dst, num Src) {
	Dst.Re *= Src;
	Dst.Im *= Src;
}

template <class type, class type2> // multiplication by another complex number
inline void operator *=(Cmpx<type> &Dst, Cmpx<type2> Src) {
	type Re = Dst.Re * Src.Re - Dst.Im * Src.Im;
	type Im = Dst.Re * Src.Im + Dst.Im * Src.Re;
	Dst.Re = Re;
	Dst.Im = Im;
}

template <class type, class num> // division by a scalar
inline void operator /=(Cmpx<type> &Dst, num Src) {
	Dst.Re /= Src;
	Dst.Im /= Src;
}

template <class type> // scalar multiplication
inline double operator *(Cmpx<type> &Left, Cmpx<type> &Right) {
	return Left.Re * Right.Re + Left.Im * Right.Im;
}

// some arithmetic functions:

// scalar product of two vectors
template <class typeA, class typeB>
inline double ScalProd(Cmpx<typeA> &A, Cmpx<typeB> &B) {
	return A.Re * B.Re + A.Im * B.Im;
}

template <class typeA, class typeB>
inline double ScalProd(typeA Ia, typeA Qa, Cmpx<typeB> &B) {
	return Ia * B.Re + Qa*B.Im;
}

// complex multiply
template <class typeDst, class typeA, class typeB>
inline void CmpxMultAxB(Cmpx<typeDst> &Dst, Cmpx<typeA> &A, Cmpx<typeB> &B) {
	Dst.Re = A.Re * B.Re - A.Im * B.Im;
	Dst.Im = A.Re * B.Im + A.Im * B.Re;
}

template <class typeDst, class typeA, class typeB>
inline void CmpxMultAxB(typeDst &DstI, typeDst &DstQ, Cmpx<typeA> &A, Cmpx<typeB> &B) {
	DstI = A.Re * B.Re - A.Im * B.Im;
	DstQ = A.Re * B.Im + A.Im * B.Re;
}

// complex multiply, second argument with a "star" (B.im is negated)
// (effectively subtracts the phase of the second argument)
template <class typeDst, class typeA, class typeB>
inline void CmpxMultAxBs(Cmpx<typeDst> &Dst, Cmpx<typeA> &A, Cmpx<typeB> &B) {
	Dst.Re = A.Re * B.Re + A.Im * B.Im;
	Dst.Im = A.Im * B.Re - A.Re * B.Im;
}

template <class Type>
void CmpxSqrt(Cmpx<Type> &X) {
	Type Mag = X.Mag();
	int NegIm = (X.Im < 0);
	X.Im = sqrt((Mag - X.Re) / 2);
	X.Re = sqrt((Mag + X.Re) / 2);
	if (NegIm) X.Re = (-X.Re);
}

template <class Type>
void CmpxSquare(Cmpx<Type> &X) {
	Type Re = X.Re * X.Re - X.Im * X.Im;
	X.Im = 2 * X.Re * X.Im;
	X.Re = Re;
}

// ----------------------------------------------------------------------------

#endif /* __CMPX_H__ */
