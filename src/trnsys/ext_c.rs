#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused)]

use std::os::raw::{c_char, c_double, c_int};

//noinspection SpellCheckingInspection
extern "C" {
    // --- Kernel subroutines ----------------------------------------------------------------------------------------------
    pub fn FOUNDBADINPUT(Input: *mut c_int, Severity: *mut c_char, Message: *mut c_char, Sevlen: usize, Messlen: usize);
    pub fn FOUNDBADPARAMETER(Param: *mut c_int, Severity: *mut c_char, Message: *mut c_char, Sevlen: usize, Messlen: usize);
    pub fn INITREPORTINTEGRAL(index: *mut c_int, intName: *mut c_char, instUnit: *mut c_char, intUnit: *mut c_char, LenName: usize, LenUnit: usize, LUnit2: usize);
    pub fn INITREPORTMINMAX(index: *mut c_int, minmaxName: *mut c_char, minmaxUnit: *mut c_char, LenName: usize, LenUnit: usize);
    pub fn INITREPORTTEXT(index: *mut c_int, txtName: *mut c_char, txtVal: *mut c_char, LenName: usize, LenVal: usize);
    pub fn INITREPORTVALUE(index: *mut c_int, valName: *mut c_char, valVal: *mut c_double, valUnit: *mut c_char, LenName: usize, LenUnit: usize);
    pub fn READNEXTCHAR(lun: *mut c_int) -> c_int;
    pub fn SETDESIREDDISCRETECONTROLSTATE(i: *mut c_int, j: *mut c_int);
    pub fn SETDYNAMICARRAYINITIALVALUE(i: *mut c_int, Value: *mut c_double);
    pub fn SETDYNAMICARRAYVALUETHISITERATION(i: *mut c_int, Value: *mut c_double);
    pub fn SETINPUTUNITS(i: *mut c_int, String: *mut c_char, len: usize);
    pub fn SETITERATIONMODE(i: *mut c_int);
    pub fn SETNUMBEROFDERIVATIVES(i: *mut c_int);
    pub fn SETNUMBEROFDISCRETECONTROLS(i: *mut c_int);
    pub fn SETNUMBEROFINPUTS(i: *mut c_int);

    pub fn SETNUMBEROFOUTPUTS(i: *mut c_int);
    pub fn SETNUMBEROFPARAMETERS(i: *mut c_int);
    pub fn SETNUMBEROFREPORTVARIABLES(nInt: *mut c_int, nMinMax: *mut c_int, nVals: *mut c_int, nText: *mut c_int);
    pub fn SETNUMBERSTOREDVARIABLES(Nrequested_Static: *mut c_int, Nrequested_Dynamic: *mut c_int);
    pub fn SETNUMERICALDERIVATIVE(i: *mut c_int, Value: *mut c_double);
    pub fn SETOUTPUTUNITS(i: *mut c_int, String: *mut c_char, len: usize);
    pub fn SETOUTPUTVALUE(i: *mut c_int, Value: *mut c_double);
    pub fn SETSTATICARRAYVALUE(i: *mut c_int, Value: *mut c_double);
    pub fn SETTYPEVERSION(i: *mut c_int) -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_ERRORFOUND() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETCONVERGENCETOLERANCE() -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETCURRENTTYPE() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETCURRENTUNIT() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETDECKFILENAME(dck: *mut c_char, len: usize) -> *mut c_char;
    pub fn TRNSYSFUNCTIONS_mp_GETDYNAMICARRAYVALUELASTTIMESTEP(i: *mut c_int) -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETFORMAT(label: *mut c_char, llen: usize, iunit: *mut c_int, no: *mut c_int) -> *mut c_char;
    pub fn TRNSYSFUNCTIONS_mp_GETINPUTVALUE(i: *mut c_int) -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETISENDOFTIMESTEP() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETISFIRSTCALLOFSIMULATION() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETISINCLUDEDINSSR() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETISLASTCALLOFSIMULATION() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETISREREADPARAMETERS() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETISSTARTTIME() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETISVERSIONSIGNINGTIME() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETLABEL(label: *mut c_char, llen: usize, iunit: *mut c_int, no: *mut c_int) -> *mut c_char;
    pub fn TRNSYSFUNCTIONS_mp_GETLUFILENAME(name: *mut c_char, llen: usize, lu: *mut c_int) -> *mut c_char;
    pub fn TRNSYSFUNCTIONS_mp_GETMAXDESCRIPLENGTH() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETMAXLABELLENGTH() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETMAXPATHLENGTH() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETMINIMUMTIMESTEP() -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETNEXTAVAILABLELOGICALUNIT() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETNUMBEROFDERIVATIVES() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETNUMBEROFINPUTS() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETNUMBEROFLABELS(i: *mut c_int) -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETNUMBEROFOUTPUTS() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETNUMBEROFPARAMETERS() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETNUMERICALSOLUTION(i: *mut c_int) -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETOUTPUTVALUE(i: *mut c_int) -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETPARAMETERVALUE(i: *mut c_int) -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETPREVIOUSCONTROLSTATE(i: *mut c_int) -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETSIMULATIONSTARTTIME() -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETSIMULATIONSTOPTIME() -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETSIMULATIONTIME() -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETSIMULATIONTIMESTEP() -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETSTATICARRAYVALUE(i: *mut c_int) -> c_double;
    pub fn TRNSYSFUNCTIONS_mp_GETTIMESTEPITERATION() -> c_int;
    pub fn TRNSYSFUNCTIONS_mp_GETTRNSYSINPUTFILEDIR(dir: *mut c_char, len: usize) -> *mut c_char;
    pub fn TRNSYSFUNCTIONS_mp_GETTRNSYSROOTDIR(dir: *mut c_char, len: usize) -> *mut c_char;
    pub fn UPDATEREPORTINTEGRAL(index: *mut c_int, intVal: *mut c_double);
    pub fn UPDATEREPORTMINMAX(index: *mut c_int, newVal: *mut c_double);

    //  --- TRNSYS subroutines  ---------------------------------------------------------------

    pub fn FLUID_PROPERTIES(units: *mut c_char, prop: *mut c_double, nref: *mut c_int, itype: *mut c_int, iflagr: *mut c_int, len: usize);
    pub fn GETHORIZONTALRADIATION(Time: *mut c_double, mode_rad: *mut c_int, mode_shape: *mut c_int, rad_input: *mut c_double, rhog: *mut c_double, slope: *mut c_double, azimuth: *mut c_double, mode_track: *mut c_int, mode_tilt: *mut c_int, latitude: *mut c_double, alt: *mut c_double, shift: *mut c_double, i_solartime: *mut c_int, SolConst: *mut c_double, td1: *mut c_double, td2: *mut c_double, solar: *mut c_double, ierror_rad: *mut c_int);
    pub fn GETTILTEDRADIATION(Time: *mut c_double, rhog: *mut c_double, slope: *mut c_double, azimuth: *mut c_double, mode_track: *mut c_int, mode_tilt: *mut c_int, alt: *mut c_double, SolConst: *mut c_double, solar: *mut c_double, ierror_rad: *mut c_int);
    pub fn INTERPOLATEDATA(LUdd: *mut c_int, NINDdd: *mut c_int, NXdd: *mut c_int, NYdd: *mut c_int, Xdd: *mut c_double, Ydd: *mut c_double);
    pub fn MESSAGES(errorCode: *mut c_int, message: *mut c_char, severity: *mut c_char, unitNo: *mut c_int, typeNo: *mut c_int, n: usize, m: usize);
    pub fn MOISTAIRPROPERTIES(CurUnit: *mut c_int, CurType: *mut c_int, iunits: *mut c_int, mode: *mut c_int, wbmd: *mut c_int, psydat: *mut c_double, emode: *mut c_int, status: *mut c_int);
    pub fn SOLVEDIFFEQ(aa: *mut c_double, bb: *mut c_double, Ti: *mut c_double, Tf: *mut c_double, Tbar: *mut c_double);
    pub fn STEAM_PROPERTIES(units: *mut c_char, prop: *mut c_double, itype: *mut c_int, ierrst: *mut c_int, len: usize);
}