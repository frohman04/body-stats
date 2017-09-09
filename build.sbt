name := "body-graphs"

version := "0.1"

scalaVersion := "2.12.3"

libraryDependencies += "org.apache.poi" % "poi-ooxml" % "3.16"
// need to build by-hand to get version that works w/ Scala 2.12
//   * add 2.12.3 as additional cross version
//   * upgrade to scalaj-http 2.3.0
//   * upgrade to json4s-native 3.5.0
//   * upgrade to scalatest 3.0.4
libraryDependencies += "co.theasi" %% "plotly" % "0.2.0"
